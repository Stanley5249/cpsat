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

// Each callback runs with at most one other callback call at a time, possibly
// on a solver worker thread, and borrows its arguments only during the call.
// A nonzero return requests the solver to stop.
// response holds a serialized CpSolverResponse.
typedef int32_t (*CpsatSolutionCallback)(void* context, const uint8_t* response,
                                         size_t response_len);
typedef int32_t (*CpsatBestBoundCallback)(void* context, double bound);
// message may not be valid UTF-8.
typedef int32_t (*CpsatLogCallback)(void* context, const uint8_t* message,
                                    size_t message_len);

// One session serves one solve, because SolveWrapper is not reusable. Set the
// parameters and add callbacks before cpsat_session_solve.
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
// valid until cpsat_session_solve returns.
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

void cpsat_buffer_free(CpsatBuffer* buffer);

#ifdef __cplusplus
}  // extern "C"
#endif

#endif  // CPSAT_BRIDGE_H_
