/* Freestanding WebAssembly only: Rust's compiler builtins provide memset. */
#ifndef HONEYDEW_FREESTANDING_STRING_H
#define HONEYDEW_FREESTANDING_STRING_H
void *memset(void *, int, __SIZE_TYPE__);
#endif
