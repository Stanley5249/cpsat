// C ABI over OR-Tools' SolveWrapper and CpSatHelper.
//
// Every message crosses the ABI as serialized Protobuf bytes. Functions that
// return int32_t return CPSAT_OK or CPSAT_ERROR; on error, *error holds a UTF-8
// message. Buffers in CpsatBuffer outputs are owned by the bridge and must be
// released with cpsat_buffer_free.
//
// Preconditions for every function: pointer arguments are non-null and valid,
// byte pointers are readable for their lengths, and a session comes from
// cpsat_session_new and is not yet freed. The bridge does not check them.

#ifndef CPSAT_BRIDGE_H_
#define CPSAT_BRIDGE_H_

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define CPSAT_OK 0
#define CPSAT_ERROR 1

typedef struct CpsatBuffer {
  uint8_t* data;
  size_t len;
} CpsatBuffer;

typedef enum CpsatEventKind {
  // data/len hold a serialized CpSolverResponse.
  CPSAT_EVENT_SOLUTION = 1,
  // bound holds the new best objective bound.
  CPSAT_EVENT_BEST_BOUND = 2,
  // data/len hold one log message, which may not be valid UTF-8.
  CPSAT_EVENT_LOG = 4,
} CpsatEventKind;

// data is borrowed and valid only during the callback.
typedef struct CpsatEvent {
  uint32_t kind;
  const uint8_t* data;
  size_t len;
  double bound;
} CpsatEvent;

// Called with at most one event at a time, possibly from a solver worker
// thread. A nonzero return requests the solver to stop.
typedef int32_t (*CpsatEventFn)(void* context, const CpsatEvent* event);

// One session serves one solve, because SolveWrapper is not reusable.
typedef struct CpsatSession CpsatSession;

// Returns null if the session cannot be allocated.
CpsatSession* cpsat_session_new(void);

// Must not run concurrently with any other call on the same session.
void cpsat_session_free(CpsatSession* session);

// Safe from any thread and from inside the event callback, while the session
// is alive.
void cpsat_session_stop(CpsatSession* session);

// events is a bitwise OR of CpsatEventKind values to subscribe to. on_event
// may be null when events is 0.
int32_t cpsat_session_solve(CpsatSession* session, const uint8_t* model,
                            size_t model_len, const uint8_t* parameters,
                            size_t parameters_len, uint32_t events,
                            CpsatEventFn on_event, void* context,
                            CpsatBuffer* response, CpsatBuffer* error);

// *message is empty when the model is valid.
int32_t cpsat_model_validate(const uint8_t* model, size_t model_len,
                             CpsatBuffer* message, CpsatBuffer* error);

int32_t cpsat_model_stats(const uint8_t* model, size_t model_len,
                          CpsatBuffer* stats, CpsatBuffer* error);

int32_t cpsat_response_stats(const uint8_t* response, size_t response_len,
                             CpsatBuffer* stats, CpsatBuffer* error);

// *parameters is a SatParameters with every singular field set to its default.
int32_t cpsat_parameters_default(CpsatBuffer* parameters, CpsatBuffer* error);

void cpsat_buffer_free(CpsatBuffer* buffer);

#ifdef __cplusplus
}  // extern "C"
#endif

#endif  // CPSAT_BRIDGE_H_
