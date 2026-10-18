#include "bridge.h"

#include <climits>
#include <cstdlib>
#include <cstring>
#include <exception>
#include <mutex>
#include <string>

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
// through Guard.
template <typename Body>
int32_t Guard(CpsatBuffer* error, Body body) noexcept {
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

struct CpsatSession {
  // CP-SAT already serializes each event kind, but log and solution events can
  // come from different threads, so this mutex lets Rust see one at a time.
  std::mutex event_mutex;
  CpsatEventFn on_event = nullptr;
  void* context = nullptr;
  SolveWrapper wrapper;

  void Dispatch(const CpsatEvent& event) {
    std::lock_guard<std::mutex> lock(event_mutex);
    if (on_event(context, &event) != 0) wrapper.StopSearch();
  }
};

namespace {

class Observer : public SolutionCallback {
 public:
  explicit Observer(CpsatSession* session) : session_(session) {}

  void OnSolutionCallback() const override {
    std::string bytes;
    SharedResponse()->SerializeToString(&bytes);
    CpsatEvent event{CPSAT_EVENT_SOLUTION,
                     reinterpret_cast<const uint8_t*>(bytes.data()),
                     bytes.size(), 0.0};
    session_->Dispatch(event);
  }

 private:
  CpsatSession* session_;
};

}  // namespace

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
  response->data = nullptr;
  response->len = 0;
  return Guard(error, [&] {
    CpModelProto model_proto;
    Parse(model_proto, model, model_len, "CpModelProto");
    SatParameters parameters;
    Parse(parameters, params, params_len, "SatParameters");

    session->on_event = on_event;
    session->context = context;
    SolveWrapper& wrapper = session->wrapper;
    wrapper.SetParameters(parameters);

    // The observer must outlive Solve, because SolveWrapper keeps a reference.
    Observer observer(session);
    if (events & CPSAT_EVENT_SOLUTION) wrapper.AddSolutionCallback(observer);
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

    const CpSolverResponse result = wrapper.Solve(model_proto);
    if (events & CPSAT_EVENT_SOLUTION) wrapper.ClearSolutionCallback(observer);
    std::string bytes;
    if (!result.SerializeToString(&bytes)) {
      throw Error{"failed to serialize CpSolverResponse"};
    }
    Fill(response, bytes);
  });
}

int32_t cpsat_validate_model(const uint8_t* model, size_t model_len,
                             CpsatBuffer* message, CpsatBuffer* error) {
  message->data = nullptr;
  message->len = 0;
  return Guard(error, [&] {
    CpModelProto model_proto;
    Parse(model_proto, model, model_len, "CpModelProto");
    Fill(message, CpSatHelper::ValidateModel(model_proto));
  });
}

int32_t cpsat_model_stats(const uint8_t* model, size_t model_len,
                          CpsatBuffer* stats, CpsatBuffer* error) {
  stats->data = nullptr;
  stats->len = 0;
  return Guard(error, [&] {
    CpModelProto model_proto;
    Parse(model_proto, model, model_len, "CpModelProto");
    Fill(stats, CpSatHelper::ModelStats(model_proto));
  });
}

int32_t cpsat_response_stats(const uint8_t* response, size_t response_len,
                             CpsatBuffer* stats, CpsatBuffer* error) {
  stats->data = nullptr;
  stats->len = 0;
  return Guard(error, [&] {
    CpSolverResponse response_proto;
    Parse(response_proto, response, response_len, "CpSolverResponse");
    Fill(stats, CpSatHelper::SolverResponseStats(response_proto));
  });
}

void cpsat_buffer_free(CpsatBuffer* buffer) {
  std::free(buffer->data);
  buffer->data = nullptr;
  buffer->len = 0;
}

}  // extern "C"
