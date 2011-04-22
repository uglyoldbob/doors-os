//these are the functions and avriables that will made available to things outside of memory.c
#include "boot_info.h"

void setup_paging(struct multiboot_info *boot_info, unsigned int size);
void *malloc(unsigned int size);
void *memcopy(void* s1, const void* s2, unsigned int n);
