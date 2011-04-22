//these are the functions and avriables that will made available to things outside of memory.c
#ifdef __cplusplus
#define EXTERNC extern "C"
#else
#define EXTERNC
#endif 

#ifndef _MEMORY_H_
#define _MEMORY_H_
#include "boot_info.h"

typedef unsigned long size_t;

EXTERNC void *malloc(unsigned int size);
EXTERNC void free(void *address);
EXTERNC void *memcpy(void *s1, const void *s2, unsigned int n);
EXTERNC void *memcopy(void *s1, const void *s2, unsigned int n);
EXTERNC void *memset(void *ptr, int value, size_t num );
EXTERNC char *strcpy(char *destination, const char *source );

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

class memory
{	//this class will handle memory management in a prettier way
	public:
		memory();
		void setup_paging(struct multiboot_info *boot_info, unsigned int size);
		friend void *memcpy(void* s1, const void* s2, unsigned int n);
		friend void *memcopy(void* s1, const void* s2, unsigned int n);
		friend void *malloc(unsigned int size);
		friend void free(void *address);
			//initializes paging and memory management
	private:
		unsigned int *page_table;		//stores the address of the PDT, the heart of the paging system
		unsigned int size_tree;			//this is the size of the binary tree (in unsigned ints)
		unsigned int *single_pages;	//total memory / 0x1000 bits
																//stores which pages were allocated one at a time
		unsigned int *page_tree;			//this is the binary tree for pages
		unsigned int largest_address;//the largest address that the tree deals with
		unsigned int *page_address;
		//0-(sizeof(unsigned int*) - 1)
		 //the size of the binary tree
		//next page (sizeof(unsigned int))
		//binary tree
		//each allocation is (size_requested + sizeof(unsigned int *))
		//add sizeof(unsigned int *) to the retrieved address and return that
		//when freeing, take the address given and subtract sizeof(unsigned int*)

		void pdtEntry(unsigned int address, unsigned int table_address);
		void tableEntry(unsigned int address);
		unsigned int getBit(unsigned int bitNum, unsigned int* tree_address);
		unsigned int getAddress(unsigned int address, unsigned int *table_address, unsigned int table_size);
		void setBit(unsigned int bit_num, int stat, unsigned int *table_address);
		void setAddress(unsigned int address, int stat, unsigned int *tree_address, unsigned int table_size);
		unsigned int countPages();
		unsigned int *alloc_page(unsigned int size, unsigned int* table, unsigned int granularity);
		void free_page(unsigned int address, unsigned int size, unsigned int *table, unsigned int granularity);
		void pdt_ptd_range(unsigned int address, unsigned int length, unsigned int *table_address, unsigned int code);
		unsigned int *alloc_bytes(unsigned int bytes);
};

#endif

#endif
