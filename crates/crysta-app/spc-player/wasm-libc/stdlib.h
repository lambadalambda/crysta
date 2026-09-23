/* The few allocation calls the vendored core makes, served by Rust's
 * allocator on wasm32 (src/wasm_libc.rs). */
#ifndef SPC_WASM_STDLIB_H
#define SPC_WASM_STDLIB_H
#include <stddef.h>
void *malloc(size_t size);
void *calloc(size_t count, size_t size);
void *realloc(void *pointer, size_t size);
void free(void *pointer);
#endif
