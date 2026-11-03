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
// Output and error buffers must be distinct, writable, and initially {NULL, 0}.
// A returned buffer may be {NULL, 0}, including on error. Free every returned
// allocation even on failure, and never free it with the caller's allocator.

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

// At most one callback of a session runs at a time, possibly on a solver worker
// thread, and it borrows its arguments only during the call. A nonzero return
// requests the solver to stop.
// Callbacks must not unwind or throw across the ABI. Only cpsat_session_solve
// invokes callbacks, and all callback calls finish before it returns. Free
// never invokes them.
// response holds a serialized CpSolverResponse.
typedef int32_t (*CpsatSolutionCallback)(void* context, const uint8_t* response,
                                         size_t response_len);
typedef int32_t (*CpsatBestBoundCallback)(void* context, double bound);
// message may not be valid UTF-8.
typedef int32_t (*CpsatLogCallback)(void* context, const uint8_t* message,
                                    size_t message_len);

// One session serves one solve, because SolveWrapper is not reusable. Set the
// parameters and add callbacks before cpsat_session_solve.
// Calls on one session must be externally serialized, except that stop may
// overlap any call other than free. A callback may call stop, but must not
// configure, solve, or free its own session.
typedef struct CpsatSession CpsatSession;

// Returns null if the session cannot be allocated.
CpsatSession* cpsat_session_new(void);

// Must not run concurrently with any other call on the same session.
void cpsat_session_free(CpsatSession* session);

// Safe from any thread and from inside a callback, while the session is alive.
void cpsat_session_stop(CpsatSession* session);

int32_t cpsat_session_set_parameters(CpsatSession* session,
                                     const uint8_t* parameters,
                                     size_t parameters_len, CpsatBuffer* error);

// Callbacks of one kind run in the order they were added. context must stay
// valid until cpsat_session_solve returns. Callback function pointers must be
// non-null.
int32_t cpsat_session_add_solution_callback(CpsatSession* session,
                                            CpsatSolutionCallback callback,
                                            void* context, CpsatBuffer* error);
int32_t cpsat_session_add_best_bound_callback(CpsatSession* session,
                                              CpsatBestBoundCallback callback,
                                              void* context,
                                              CpsatBuffer* error);
int32_t cpsat_session_add_log_callback(CpsatSession* session,
                                       CpsatLogCallback callback, void* context,
                                       CpsatBuffer* error);

int32_t cpsat_session_solve(CpsatSession* session, const uint8_t* model,
                            size_t model_len, CpsatBuffer* response,
                            CpsatBuffer* error);

// *message is empty when the model is valid.
int32_t cpsat_model_validate(const uint8_t* model, size_t model_len,
                             CpsatBuffer* message, CpsatBuffer* error);

int32_t cpsat_model_stats(const uint8_t* model, size_t model_len,
                          CpsatBuffer* stats, CpsatBuffer* error);

int32_t cpsat_response_stats(const uint8_t* response, size_t response_len,
                             CpsatBuffer* stats, CpsatBuffer* error);

// *parameters is a SatParameters with every singular field set to its default.
int32_t cpsat_parameters_default(CpsatBuffer* parameters, CpsatBuffer* error);

// buffer must be {NULL, 0} or own an allocation returned by this bridge.
// Resets *buffer to {NULL, 0}, so freeing that same buffer again is harmless.
// Copies of CpsatBuffer do not acquire ownership of its allocation.
void cpsat_buffer_free(CpsatBuffer* buffer);

#ifdef __cplusplus
}  // extern "C"
#endif

#endif  // CPSAT_BRIDGE_H_
