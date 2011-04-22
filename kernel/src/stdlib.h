#ifdef __cplusplus
#define EXTERNC extern "C"
#else
#define EXTERNC
#endif 

#include <stddef.h>

EXTERNC void *malloc(size_t size);
EXTERNC void free(void *ptr);
