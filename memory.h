#ifndef MEMORY_H
#define MEMORY_H
//define a few globals used for memory management in the kernel

struct MemoryRange 
{	//for a singly linked list of available memory addresses (this will not be used for memory allocation)
	//this will be used to help setup the page directory
	unsigned long Base;		//base address in bytes
	unsigned long Length;		//length in bytes
	MemoryRange *Next;		//the next memory range (0 if last)
};	//dont forget the semicolon

	MemoryRange *First;	//the first memory range record
	MemoryRange *CurMem;	//the current memory range record
	unsigned long SizeRam;	//the amount of RAM that must be paged (for now the limit is 4GB)
	unsigned long PhyPages;	//the number of pages for PHYSICAL RAM
	unsigned long PhyTables;//self explanatory
	unsigned long VirPages;	//the number of pages that are not outside of the range of RAM (virtual memory)
	unsigned long VirTables;//self explanatory

	unsigned long Levels;	//for memory management heaps
	unsigned long Size;
	unsigned long HeaderSize;	//2^(n+1)
	unsigned long *Heap1;
	unsigned long *Heap2;

#endif
