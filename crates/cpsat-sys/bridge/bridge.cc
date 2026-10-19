#include "bridge.h"

#include <climits>
#include <cstdlib>
#include <cstring>
#include <exception>
#include <mutex>
#include <string>

#include "google/protobuf/descriptor.h"
#include "google/protobuf/message.h"
#include "ortools/sat/cp_model.pb.h"
#include "ortools/sat/sat_parameters.pb.h"
#include "ortools/sat/swig_helper.h"

namespace {

using operations_research::sat::CpModelProto;
using operations_research::sat::CpSatHelper;
using operations_research::sat::CpSolverResponse;
using operations_research::sat::SatParameters;
using operations_research::sat::SolutionCallback;
using operations_research::sat::SolveWrapper;

struct Error {
  std::string message;
};

void Fill(CpsatBuffer* out, const std::string& bytes) {
  out->data = nullptr;
  out->len = bytes.size();
  if (bytes.empty()) return;
  out->data = static_cast<uint8_t*>(std::malloc(bytes.size()));
  if (out->data == nullptr) throw std::bad_alloc();
  std::memcpy(out->data, bytes.data(), bytes.size());
}

// Serializes straight into the bridge-owned buffer, without an intermediate
// std::string.
void FillMessage(CpsatBuffer* out, const google::protobuf::Message& message) {
  const size_t size = message.ByteSizeLong();
  const std::string name(message.GetTypeName());
  out->data = nullptr;
  out->len = size;
  if (size == 0) return;
  if (size > static_cast<size_t>(INT_MAX)) throw Error{name + " exceeds 2 GiB"};
  out->data = static_cast<uint8_t*>(std::malloc(size));
  if (out->data == nullptr) throw std::bad_alloc();
  if (!message.SerializeToArray(out->data, static_cast<int>(size))) {
    throw Error{"failed to serialize " + name};
  }
}

// Sets every singular field to its schema default, so the serialized message
// carries the defaults that unset proto2 fields otherwise hide.
void SetDefaults(google::protobuf::Message& message) {
  using google::protobuf::FieldDescriptor;
  const auto* descriptor = message.GetDescriptor();
  const auto* reflection = message.GetReflection();
  for (int i = 0; i < descriptor->field_count(); ++i) {
    const FieldDescriptor* f = descriptor->field(i);
    if (f->is_repeated() || f->real_containing_oneof() != nullptr) continue;
    switch (f->cpp_type()) {
      case FieldDescriptor::CPPTYPE_INT32:
        reflection->SetInt32(&message, f, f->default_value_int32());
        break;
      case FieldDescriptor::CPPTYPE_INT64:
        reflection->SetInt64(&message, f, f->default_value_int64());
        break;
      case FieldDescriptor::CPPTYPE_UINT32:
        reflection->SetUInt32(&message, f, f->default_value_uint32());
        break;
      case FieldDescriptor::CPPTYPE_UINT64:
        reflection->SetUInt64(&message, f, f->default_value_uint64());
        break;
      case FieldDescriptor::CPPTYPE_DOUBLE:
        reflection->SetDouble(&message, f, f->default_value_double());
        break;
      case FieldDescriptor::CPPTYPE_FLOAT:
        reflection->SetFloat(&message, f, f->default_value_float());
        break;
      case FieldDescriptor::CPPTYPE_BOOL:
        reflection->SetBool(&message, f, f->default_value_bool());
        break;
      case FieldDescriptor::CPPTYPE_ENUM:
        reflection->SetEnumValue(&message, f,
                                 f->default_value_enum()->number());
        break;
      case FieldDescriptor::CPPTYPE_STRING:
        reflection->SetString(&message, f,
                              std::string(f->default_value_string()));
        break;
      case FieldDescriptor::CPPTYPE_MESSAGE:
        // A nested message has no schema default.
        break;
    }
  }
}

template <typename Message>
void Parse(Message& message, const uint8_t* data, size_t len,
           const char* name) {
  if (len > static_cast<size_t>(INT_MAX)) {
    throw Error{std::string(name) + " exceeds 2 GiB"};
  }
  if (!message.ParseFromArray(data, static_cast<int>(len))) {
    throw Error{std::string("failed to parse ") + name};
  }
}

// Leaves *error empty when even the message cannot be allocated, which Rust
// reports as an unknown error.
void SetError(CpsatBuffer* error, const std::string& message) noexcept {
  try {
    Fill(error, message);
  } catch (...) {
    error->data = nullptr;
    error->len = 0;
  }
}

// Exceptions must not cross the C ABI, so every entry point runs its body
// through Guard, which also empties both output buffers first.
template <typename Body>
int32_t Guard(CpsatBuffer* out, CpsatBuffer* error, Body body) noexcept {
  out->data = nullptr;
  out->len = 0;
  error->data = nullptr;
  error->len = 0;
  try {
    body();
    return CPSAT_OK;
  } catch (const Error& e) {
    SetError(error, e.message);
  } catch (const std::exception& e) {
    SetError(error, e.what());
  } catch (...) {
    SetError(error, "unknown C++ exception");
  }
  return CPSAT_ERROR;
}

}  // namespace

namespace {

class Observer : public SolutionCallback {
 public:
  explicit Observer(CpsatSession* session) : session_(session) {}
  void OnSolutionCallback() const override;

 private:
  CpsatSession* session_;
};

}  // namespace

struct CpsatSession {
  // CP-SAT already serializes each event kind, but log and solution events can
  // come from different threads, so this mutex lets Rust see one at a time.
  // Rust relies on it to hold the only live reference to its callback state.
  std::mutex event_mutex;
  CpsatEventFn on_event = nullptr;
  void* context = nullptr;
  bool used = false;
  SolveWrapper wrapper;
  // SolveWrapper keeps a reference to the observer, so the observer lives as
  // long as the session rather than as long as one call.
  Observer observer{this};

  void Dispatch(const CpsatEvent& event) {
    std::lock_guard<std::mutex> lock(event_mutex);
    if (on_event(context, &event) != 0) wrapper.StopSearch();
  }
};

void Observer::OnSolutionCallback() const {
  std::string bytes;
  SharedResponse()->SerializeToString(&bytes);
  session_->Dispatch(CpsatEvent{CPSAT_EVENT_SOLUTION,
                                reinterpret_cast<const uint8_t*>(bytes.data()),
                                bytes.size(), 0.0});
}

extern "C" {

CpsatSession* cpsat_session_new(void) {
  try {
    return new CpsatSession();
  } catch (...) {
    return nullptr;
  }
}

void cpsat_session_free(CpsatSession* session) { delete session; }

void cpsat_session_stop(CpsatSession* session) {
  try {
    session->wrapper.StopSearch();
  } catch (...) {
    // StopSearch only sets a flag under a mutex, so a failure here leaves
    // nothing to report and the solve simply runs to its own limits.
  }
}

int32_t cpsat_session_solve(CpsatSession* session, const uint8_t* model,
                            size_t model_len, const uint8_t* params,
                            size_t params_len, uint32_t events,
                            CpsatEventFn on_event, void* context,
                            CpsatBuffer* response, CpsatBuffer* error) {
  return Guard(response, error, [&] {
    if (session->used) throw Error{"a CP-SAT session serves one solve"};
    session->used = true;
    CpModelProto model_proto;
    Parse(model_proto, model, model_len, "CpModelProto");
    SatParameters parameters;
    Parse(parameters, params, params_len, "SatParameters");

    session->on_event = on_event;
    session->context = context;
    SolveWrapper& wrapper = session->wrapper;
    wrapper.SetParameters(parameters);

    if (events & CPSAT_EVENT_SOLUTION) {
      wrapper.AddSolutionCallback(session->observer);
    }
    if (events & CPSAT_EVENT_BEST_BOUND) {
      wrapper.AddBestBoundCallback([session](double bound) {
        session->Dispatch(CpsatEvent{CPSAT_EVENT_BEST_BOUND, nullptr, 0, bound});
      });
    }
    if (events & CPSAT_EVENT_LOG) {
      wrapper.AddLogCallback([session](const std::string& message) {
        session->Dispatch(CpsatEvent{
            CPSAT_EVENT_LOG, reinterpret_cast<const uint8_t*>(message.data()),
            message.size(), 0.0});
      });
    }

    // Solve joins its worker threads before it returns, so every event
    // callback happens before the caller reads its callback state.
    FillMessage(response, wrapper.Solve(model_proto));
  });
}

int32_t cpsat_model_validate(const uint8_t* model, size_t model_len,
                             CpsatBuffer* message, CpsatBuffer* error) {
  return Guard(message, error, [&] {
    CpModelProto model_proto;
    Parse(model_proto, model, model_len, "CpModelProto");
    Fill(message, CpSatHelper::ValidateModel(model_proto));
  });
}

int32_t cpsat_model_stats(const uint8_t* model, size_t model_len,
                          CpsatBuffer* stats, CpsatBuffer* error) {
  return Guard(stats, error, [&] {
    CpModelProto model_proto;
    Parse(model_proto, model, model_len, "CpModelProto");
    Fill(stats, CpSatHelper::ModelStats(model_proto));
  });
}

int32_t cpsat_response_stats(const uint8_t* response, size_t response_len,
                             CpsatBuffer* stats, CpsatBuffer* error) {
  return Guard(stats, error, [&] {
    CpSolverResponse response_proto;
    Parse(response_proto, response, response_len, "CpSolverResponse");
    Fill(stats, CpSatHelper::SolverResponseStats(response_proto));
  });
}

int32_t cpsat_parameters_default(CpsatBuffer* params, CpsatBuffer* error) {
  return Guard(params, error, [&] {
    SatParameters parameters;
    SetDefaults(parameters);
    FillMessage(params, parameters);
  });
}

void cpsat_buffer_free(CpsatBuffer* buffer) {
  std::free(buffer->data);
  buffer->data = nullptr;
  buffer->len = 0;
}

}  // extern "C"
