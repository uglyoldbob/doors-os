#ifdef __cplusplus
#define EXTERNC extern "C"
#else
#define EXTERNC
#endif 

#include <stdlib.h>
#include <stddef.h>

#include "memory.h"

EXTERNC void *malloc(size_t size)
{
	kmalloc(size);
}

EXTERNC void free(void *ptr)
{
	kfree(ptr);
}
