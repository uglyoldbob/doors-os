//these are the functions and avriables that will made available to things outside of memory.c
#ifdef __cplusplus
#define EXTERNC extern "C"
#else
#define EXTERNC
#endif 

#ifndef _MEMORY_H_
#define _MEMORY_H_
#include "boot_info.h"
#include <stddef.h>
#include "entrance.h"

//EXTERNC void free(void *address);

EXTERNC void *kmalloc(size_t size);
EXTERNC void kfree(void *address);

EXTERNC void *memcpy(void *s1, const void *s2, size_t n);
EXTERNC void *memcopy(void *s1, const void *s2, size_t n);

EXTERNC void setup_paging(struct multiboot_info *boot_info, size_t size);

struct page_range
{
	struct page_range *previous;
	unsigned int address;
	unsigned int length;	//number of pages
	struct page_range *next;
};


#ifdef __cplusplus
//overload the operator "new"
void * operator new (size_t size);


//overload the operator "new[]"
void * operator new[] (size_t size);


//overload the operator "delete"
void operator delete (void * p);


//overload the operator "delete[]"
void operator delete[] (void * p);
#endif

#endif
