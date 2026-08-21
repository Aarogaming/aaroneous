#pragma once
#include <cstdint>

// AAS-ABI: The Universal Binary Interface (Enzymatic Contract)
// This interface allows Aaroneous Kernel to bind to any machine-native binary.

#ifdef __cplusplus
extern "C" {
#endif

// Universal status codes for all enzyme operations
typedef enum {
    AAS_OK = 0,
    AAS_ERROR_GENERIC = 1,
    AAS_ERROR_MEMORY_ACCESS = 2,
    AAS_ERROR_MALFORMED_CHROMOSOME = 3
} aas_status;

// Zero-copy buffer structure for high-speed tensor access (The Synapse)
typedef struct {
    void* data;
    uint64_t size;
    uint64_t capacity;
} aas_buffer;

// Standard enzyme entry points (Exported symbols for binary loaders)
typedef aas_status (*aas_init_func)();
typedef aas_status (*aas_process_func)(aas_buffer* input, aas_buffer* output);
typedef aas_status (*aas_shutdown_func)();

#ifdef __cplusplus
}
#endif
