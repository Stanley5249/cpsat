#include "bridge.h"

#include <climits>
#include <cstdlib>
#include <cstring>
#include <exception>
#include <list>
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

// Points *out at size bridge-owned bytes, or at nothing when size is 0.
uint8_t* Allocate(CpsatBuffer* out, size_t size) {
  *out = CpsatBuffer{};
  if (size == 0) return nullptr;
  auto* data = static_cast<uint8_t*>(std::malloc(size));
  if (data == nullptr) throw std::bad_alloc();
  *out = CpsatBuffer{data, size};
  return data;
}

void Fill(CpsatBuffer* out, const std::string& bytes) {
  if (uint8_t* data = Allocate(out, bytes.size())) {
    std::memcpy(data, bytes.data(), bytes.size());
  }
}

// Serializes straight into the bridge-owned buffer, without an intermediate
// std::string.
void FillMessage(CpsatBuffer* out, const google::protobuf::Message& message) {
  const size_t size = message.ByteSizeLong();
  if (size > static_cast<size_t>(INT_MAX)) {
    throw Error{std::string(message.GetTypeName()) + " exceeds 2 GiB"};
  }
  uint8_t* data = Allocate(out, size);
  if (data != nullptr &&
      !message.SerializeToArray(data, static_cast<int>(size))) {
    throw Error{"failed to serialize " + std::string(message.GetTypeName())};
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
    *error = CpsatBuffer{};
  }
}

// Exceptions must not cross the C ABI, so every entry point runs its body
// through Guard, which also empties the error buffer first.
template <typename Body>
int32_t Guard(CpsatBuffer* error, Body body) noexcept {
  *error = CpsatBuffer{};
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

// Also empties the output buffer first.
template <typename Body>
int32_t Guard(CpsatBuffer* out, CpsatBuffer* error, Body body) noexcept {
  *out = CpsatBuffer{};
  return Guard(error, body);
}

class Observer : public SolutionCallback {
 public:
  Observer(CpsatSession* session, CpsatSolutionCallback callback,
           void* context)
      : session_(session), callback_(callback), context_(context) {}
  void OnSolutionCallback() const override;

 private:
  CpsatSession* session_;
  CpsatSolutionCallback callback_;
  void* context_;
};

}  // namespace

struct CpsatSession {
  // CP-SAT already serializes each callback kind, but log and solution
  // callbacks can run on different threads, so this mutex lets Rust see one
  // call at a time. Rust relies on it to hold the only live reference to each
  // callback's state.
  std::mutex callback_mutex;
  bool used = false;
  // SolveWrapper keeps a reference to each observer, so the observers sit in a
  // list whose elements never move, declared first so they outlive it.
  std::list<Observer> observers;
  SolveWrapper solve_wrapper;

  template <typename Call>
  void Dispatch(Call call) {
    std::lock_guard<std::mutex> lock(callback_mutex);
    if (call() != 0) solve_wrapper.StopSearch();
  }

  void CheckUnused() const {
    if (used) throw Error{"a CP-SAT session serves one solve"};
  }
};

void Observer::OnSolutionCallback() const {
  std::string bytes;
  SharedResponse()->SerializeToString(&bytes);
  session_->Dispatch([&] {
    return callback_(context_, reinterpret_cast<const uint8_t*>(bytes.data()),
                     bytes.size());
  });
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
    session->solve_wrapper.StopSearch();
  } catch (...) {
    // StopSearch only sets a flag under a mutex, so a failure here leaves
    // nothing to report and the solve simply runs to its own limits.
  }
}

int32_t cpsat_session_set_parameters(CpsatSession* session,
                                     const uint8_t* parameters,
                                     size_t parameters_len,
                                     CpsatBuffer* error) {
  return Guard(error, [&] {
    session->CheckUnused();
    SatParameters parameters_proto;
    Parse(parameters_proto, parameters, parameters_len, "SatParameters");
    session->solve_wrapper.SetParameters(parameters_proto);
  });
}

int32_t cpsat_session_add_solution_callback(CpsatSession* session,
                                            CpsatSolutionCallback callback,
                                            void* context, CpsatBuffer* error) {
  return Guard(error, [&] {
    session->CheckUnused();
    session->solve_wrapper.AddSolutionCallback(
        session->observers.emplace_back(session, callback, context));
  });
}

int32_t cpsat_session_add_best_bound_callback(CpsatSession* session,
                                              CpsatBestBoundCallback callback,
                                              void* context,
                                              CpsatBuffer* error) {
  return Guard(error, [&] {
    session->CheckUnused();
    session->solve_wrapper.AddBestBoundCallback(
        [session, callback, context](double bound) {
          session->Dispatch([&] { return callback(context, bound); });
        });
  });
}

int32_t cpsat_session_add_log_callback(CpsatSession* session,
                                       CpsatLogCallback callback, void* context,
                                       CpsatBuffer* error) {
  return Guard(error, [&] {
    session->CheckUnused();
    session->solve_wrapper.AddLogCallback(
        [session, callback, context](const std::string& message) {
          session->Dispatch([&] {
            return callback(context,
                            reinterpret_cast<const uint8_t*>(message.data()),
                            message.size());
          });
        });
  });
}

int32_t cpsat_session_solve(CpsatSession* session, const uint8_t* model,
                            size_t model_len, CpsatBuffer* response,
                            CpsatBuffer* error) {
  return Guard(response, error, [&] {
    session->CheckUnused();
    session->used = true;
    CpModelProto model_proto;
    Parse(model_proto, model, model_len, "CpModelProto");
    // Solve joins its worker threads before it returns, so every callback
    // happens before the caller reads its callback state.
    FillMessage(response, session->solve_wrapper.Solve(model_proto));
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

int32_t cpsat_parameters_default(CpsatBuffer* parameters, CpsatBuffer* error) {
  return Guard(parameters, error, [&] {
    SatParameters parameters_proto;
    SetDefaults(parameters_proto);
    FillMessage(parameters, parameters_proto);
  });
}

void cpsat_buffer_free(CpsatBuffer* buffer) {
  std::free(buffer->data);
  *buffer = CpsatBuffer{};
}

}  // extern "C"
