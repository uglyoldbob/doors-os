
//memory.c

//this should only be used for kernel memory management
//user level memory management needs another solution
//TODO: implement a method so that memory can be obtained from a certain location
	//meaning...
	//a memory address might have to be located in the bottom of memory to satisfy limitations of hardware (such as DMA that only goes to the bottom megabyte (or two?)
	//a memory address that will be used for paging structures

	unsigned int *page_table;		//stores the address of the PDT, the heart of the paging system

//page level allocation
	unsigned int size_tree;			//this is the size of the binary tree (in unsigned ints)
	unsigned int *single_pages;		//total memory / PAGE_SIZE bits
											//stores which pages were allocated one at a time
	unsigned int *page_tree;			//this is the binary tree for pages
	unsigned int largest_address;	//the largest address that the tree deals with

//less than page size level allocation
	unsigned int *page_address;		//used for allocations less than PAGE_SIZE
	//format of this consists of a length
		//top most bit (0x800000000) defines of that segment is in use (has been allocated)
	//each allocation is (size_requested + sizeof(unsigned int *))
	//add BYTE_GRANULARITY to the retrieved address and return that
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


/*
paging is enabled here
the paging structures are assigned 1:1
physical addresses are given for all paging structures
physical memory addresses = virtual memory address (at least for now)
memory allocation for single pages uses a binary tree for allocation and then half a binary tree for deallocation
for less than one page of allocation, a page is allocated (the last 4 bytes are the address of the next page)
	the next to last 4 bytes are 0 for reference purposes
	allocate page uses first fit
	each memory range specifies length of the segment and whether or not it is being used currently
	kmalloc searches for best fit in this case
for allocating more than one page (it's pretty slow i think)
	it scans the bottom layer of the binary tree and stores information about the best fit memory block
	then it marks that memory block as used and returns the address
	this is also a best fit algorithm, but it is quite slow
	when a memory block is added, it is added to a linked list of (address/length pairs)
	when a memory block is removed, the linked list is scanned for that address,
		then the memory is freed and that particular element is removed from the linked list
*/

#include "boot_info.h"
#include "memory.h"
#include "video.h"
#include "entrance.h"
#include "spinlock.h"

//memory global_memory;

//page granularity binary tree
//1 - indicate the presence of a free page
	//will be used to allocate >= 1 page

const unsigned int BYTE_GRANULARITY = sizeof(unsigned int *);
	//this is the granularity of an allocation that is less than 4KB
const unsigned int PAGE_SIZE = 0x1000;
	//define the size of a page and eliminate dependencies on pages being a certain size

struct page_range *first_pages_range;	//the last element in this linked list will be 0,0,0
	//this linked list stores ??????


void pdtEntry(unsigned int address, unsigned int table_address)
{	//fills in the entry in the PDT regarding the address (0-4GB)
	//it will point to table_address for that memory range
	page_table[address] = table_address + 3; //read/write, present, supervisor
}

void tableEntry(unsigned int address)
{	//fills in the page table entry for address
	//retrieves the page table address from the page directory table
	unsigned int *table_address = (unsigned int *)(page_table[address / 0x400000] & 0xFFFFF000);
	table_address[(address / PAGE_SIZE) % 0x400] = address + 3;
}

unsigned int getBit(unsigned int bitNum, unsigned int* tree_address)
{	//this retrieves the bitNum'th bit from the binary tree at [tree_address] 
	unsigned int value;
	value = tree_address[bitNum /(sizeof(unsigned int) * 8)] & 1<<(bitNum % (sizeof(unsigned int) * 8));
	value = value>>(bitNum % (sizeof(unsigned int) * 8));
	return value;
}

unsigned int getAddress(unsigned int address, unsigned int *table_address, unsigned int table_size)
{	//table_size is size in bytes
	//table_address is the address of the binary tree
	return getBit(table_size * sizeof(unsigned int) + address, table_address);
}

void setBit(unsigned int bit_num, int stat, unsigned int *table_address)
{	//sets the nth bit in the binary tree at table_address to stat (1 or 0)
	if (stat)
	{
		table_address[bit_num /(sizeof(unsigned int) * 8)] |= 
			1<<(bit_num % (sizeof(unsigned int) * 8));
	}
	else
	{
		table_address[bit_num /(sizeof(unsigned int) * 8)] &= 
			~(1<<(bit_num % (sizeof(unsigned int) * 8)));
	}
}

void setAddress(unsigned int address, int stat, unsigned int *tree_address, unsigned int table_size)
{	//makes a setBit call after it does math to find the proper bit to set
	//caller is required to divide address by the granularity achieved by the table (4 KB = 0x1000);
	//table_size = bytes
	//bit = table_size * 4 + address
	setBit(table_size * sizeof(unsigned int) + address, stat, tree_address);
}

unsigned int countPages()
{	//returns the number of usable pages
	unsigned int pages = 0;
	unsigned int counter;
	for (counter = 0; counter < largest_address; counter += PAGE_SIZE)
	{
		if (getAddress(counter / PAGE_SIZE, page_tree, size_tree * sizeof(unsigned int)))
		{
			pages++;
		}
	}
	return pages;
}

unsigned int *alloc_page(unsigned int size, unsigned int* table, unsigned int granularity)
{	//returns the address of 1 free page
	//size is the size of the tree in bytes
	unsigned int unused_page = 1;
	while (unused_page < (size * 0x4))
	{
		if (getBit(unused_page * 2, table))
		{
			unused_page *= 2;
		}
		else if (getBit(unused_page * 2 + 1, table))
		{
			unused_page = unused_page * 2 + 1;
		}
		else
		{
			return 0;	//error
		}
	}
	//scan back up the table and adjust everything
	unsigned int counter;
	setBit(unused_page, 0, table);
	for (counter = unused_page / 2; counter >= 1; counter /= 2)
	{
		setBit(counter, getBit(2 * counter, table) | getBit(2 * counter + 1, table), table);
	}
	unused_page = (unused_page - (size * 0x4)) * granularity;
	//address / granularity + size * 0x4 = bitNum
	return (unsigned int *)unused_page;
}

void free_page(unsigned int address, unsigned int size, unsigned int *table, unsigned int granularity)
{	//frees 1 page and updates the binary tree
	//size is the size of the table in bytes, 4 is the constant to get the bit number of the first bit in the last row from bytes
	unsigned int bit_num = size * 0x4 + address / granularity;
	unsigned int counter;
	setAddress(address / granularity, 1, table, size);
	for (counter = bit_num / 2; counter >= 1; counter /= 2)
	{
		setBit(counter, getBit(2 * counter, table) | getBit(2 * counter + 1, table), table);
	}
}

void pdt_ptd_range(unsigned int address, unsigned int length, unsigned int *table_address, unsigned int code)
{	//fills out the PDT, and PTE for a given memory range
//0, 0x100000, &table_address, 2
	unsigned int pdt_calc;
	for (pdt_calc = address / 0x400000;
			pdt_calc <= ((address + length - 1) / 0x400000);
			pdt_calc++)
	{	//fill in all entries for each new 4MB range found
		if (page_table[pdt_calc] == 0)
		{	//only change stuff if this entry has not been entered
			pdtEntry(pdt_calc, *table_address);
			*table_address = *table_address + PAGE_SIZE;	//ok so the next table goes on the next page
			//fill out the PageTableDirectory we just assigned
		}
	}
	for (pdt_calc = address - (address % PAGE_SIZE);
			 pdt_calc <= (address + length); pdt_calc+= PAGE_SIZE)
	{	//starts at the first page boundary at or before the memory range starts
		//goes up page by page until it reaches the end of the memory range
		//fill in page at counter
		if (pdt_calc < (address - (address % PAGE_SIZE)))
			break;	//catch/stop any overflow bugs from address 0xFFFFE000
		tableEntry(pdt_calc);
		//determine if memory is in range
	}
	if (code == 1)
	{	//find the largest memory address that can possibly be allocated
		largest_address = address + length - 1;
	}
}

unsigned int *alloc_bytes(unsigned int bytes)
{	//searches for number_bytes of unused bytes
	//then adds it the the list of assigned addresses
	//returns the address number_bytes of contiguous memory (at least)
	//size is the size of the tree in bytes

	//page = chunk address
	//*page = length
	//page + *page = next chunk

	unsigned int number_bytes;
	if ((bytes % BYTE_GRANULARITY) == 0)
		number_bytes = bytes;
	else
		number_bytes = bytes + BYTE_GRANULARITY - (bytes % BYTE_GRANULARITY);
		//align the length to an appropriate value
	if ((number_bytes + BYTE_GRANULARITY) >= PAGE_SIZE)
		return 0;
	unsigned int address = 0;
	unsigned int length = 0xFFFFFFFF;	//the length of the current best fit area
	unsigned int *page = page_address;
	unsigned int *new_page;	//this is so we know where to store the address if another page is required
	do
	{
		for (; *page != 0; page += ((*page & 0x7FFFFFFF) / BYTE_GRANULARITY))
		{
			if (!(*page & 0x80000000))
			{	//only if this segment is unused
				if (*page >= (number_bytes + BYTE_GRANULARITY))
				{	//only if the segment is long enough
					if (*page <= length)
					{	//length_requested <= this_length <= previous_length (best fit algorithm)
						length = *page;
						address = (unsigned int)page;
					}
				}
			}
		}
		page += 1;
		new_page = page;
		page = (unsigned int*)*page;
	} while (page	!= 0);
	//now that the best fit has been found, modify appropriately
	if (address == 0)
	{	//allocate another page for allocating memory
		*new_page = (unsigned int)alloc_page(size_tree * 4, page_tree, PAGE_SIZE);
			//(unsigned int)kmalloc(PAGE_SIZE);	
			//set the address for the previous page so that this page will be included in the search
		new_page = (unsigned int *)*new_page;
		if (new_page == 0)
		{
			return 0;	//indicate that there is no more memory to allocate]
		}
		//initialize the new page
		new_page[0] = 0x80000000 + number_bytes + BYTE_GRANULARITY;
		page = (unsigned int*)(number_bytes + BYTE_GRANULARITY + (unsigned int)new_page);
		*page = PAGE_SIZE - 2 * BYTE_GRANULARITY - number_bytes - BYTE_GRANULARITY;
		new_page[PAGE_SIZE / BYTE_GRANULARITY - 2] = 0;
		new_page[PAGE_SIZE / BYTE_GRANULARITY - 1] = 0;
		return (unsigned int *)(new_page + BYTE_GRANULARITY);
	}
	if (length <= (number_bytes + 3 * BYTE_GRANULARITY))
	{	//then all you have to do is mark it as used because there is not enough free space for another segment
		*(unsigned int*)address += 0x80000000; //always length < PAGE_SIZE
	}	//a free segment has to be at least 2*BYTE_GRANULARITY
	else
	{	//then modification is required (plenty of room for a blank segment after the end of the new used segment
		page = (unsigned int*)(number_bytes + BYTE_GRANULARITY + address);
		*page = length - number_bytes - BYTE_GRANULARITY;
		*(unsigned int*)address = 0x80000000 + number_bytes + BYTE_GRANULARITY;
	}
	return (unsigned int *)(address + BYTE_GRANULARITY);
}

//overload the operator "new"
void * operator new (size_t size)
{
    return kmalloc(size);
}

//overload the operator "new[]"
void * operator new[] (size_t size)
{
    return kmalloc(size);
}

//overload the operator "delete"
void operator delete (void * p)
{
    kfree(p);
}

//overload the operator "delete[]"
void operator delete[] (void * p)
{
    kfree(p);
}

void chart_memory()
{
	unsigned int counter;
	unsigned int *page;	
	display("Page availability chart:\n");
	for (counter = 0; counter < 0x3FFF; counter += PAGE_SIZE)
	{
		if (getAddress(counter / PAGE_SIZE, page_tree, size_tree * sizeof(unsigned int)))
		{
			display("1");
		}
		else
			display("0");
	}
	display("\n");
	page = page_address;
	do
	{
		for (; *page != 0; page += ((*page & 0x7FFFFFFF) / BYTE_GRANULARITY))
		{
			if (!(*page & 0x80000000))
			{	//only if this segment is unused
				PrintNumber((unsigned long)page);
				display(",");
				PrintNumber(*page & 0x7FFFFFFF);
				display(" not used.\t");
			}
			else
			{
				PrintNumber((unsigned long)page);
				display(",");
				PrintNumber(*page & 0x7FFFFFFF);
				display(" used\t");
			}
		}
		page += 1;
		page = (unsigned int*)*page;
	} while (page	!= 0);
	display("\n");
}

void *kmalloc(size_t size)
{	//size is in bytes
//	PrintNumber(getCallerEIP(0x4C));
//	display("\t");
	enter_spinlock(SL_MEM_MNG);
	//check all memory management structures and find out when they take a dump	
	void *use_me = 0;
	unsigned int actual_size;
	unsigned int address_search;	//scans each address
	unsigned int address_current;	//stores the current address and length
	unsigned int length_current;
	unsigned int counter;
	unsigned int counter2;
	unsigned int address = 0;		//address and length (in pages) of the current match
	unsigned int length = 0xFFFFFFFF;
	struct page_range *fill_me;
	if (size == 0)
		size = 1;
	if (size == PAGE_SIZE)
	{	//allocate en exact page from the allocation tree
		use_me = alloc_page(size_tree * 4, page_tree, PAGE_SIZE);
		setBit((unsigned int)use_me / PAGE_SIZE, 1, single_pages);
		leave_spinlock(SL_MEM_MNG);
//display("Return: ");
//PrintNumber((unsigned long)use_me);
//display("\n");
		return use_me;
	}
	else if ((size + BYTE_GRANULARITY) < PAGE_SIZE)
	{	//requires "special" allocation
		//find the closest group of memory that is large enough
		//if no open spots are large enough, go to the next page
		//if it is the last page, then allocate another page
		use_me = alloc_bytes(size);
		leave_spinlock(SL_MEM_MNG);
//display("Return: ");
//PrintNumber((unsigned long)use_me);
//display("\n");
		return use_me;
	}
	else
	{	//size > PAGE_SIZE (or the addition of byte level allocation requires page level allocation)
		//search memory and find the best fit memory block
		//allocations are stored in a linked list
		if ((size % PAGE_SIZE) == 0)
		{	//round up to the nearest 4kb block (even if 1 byte over)
			actual_size = size;
		}
		else
		{
			actual_size = size + (PAGE_SIZE - (size % PAGE_SIZE));
		}
		actual_size /= PAGE_SIZE;	//number of contiguous pages needed
		address_current = address_search;
		length_current = 0;
		for (address_search = 0; address_search < largest_address; address_search += PAGE_SIZE)
		{
			if (getAddress(address_search / PAGE_SIZE,
											page_tree,
											size_tree * sizeof(unsigned int)) == 1)
			{
				length_current++;
			}
			else
			{
				//does this block fit better than the one already found?
				if (length_current < length)
				{
					if (length_current >= actual_size)
					{	//it fits better
						length = length_current;
						address = address_current;
						if (length_current == actual_size)
							break;
					}
				}
				length_current = 0;
				address_current = address_search + PAGE_SIZE;
			}
		}
		if (length == 0xFFFFFFFF)
		{
			leave_spinlock(SL_MEM_MNG);
//display("Return: ");
//PrintNumber((unsigned long)use_me);
//display("\n");
			return 0;	//the whole of memory cannot be allocated right now
		}
		for (address_search = address; address_search < ((address + (PAGE_SIZE * actual_size)) - 1); address_search += PAGE_SIZE)
		{
			setAddress(address_search / PAGE_SIZE, 0, page_table, size_tree * sizeof(unsigned int));
		}
		//update the entire tree
		for (counter = size_tree * 0x8; counter >= 1; counter /= 2)
		{	//have to start on the second lowest layer
			for (counter2 = counter; counter2 < (counter * 2); counter2++)
			{
				setBit(counter2,
									getBit(2 * counter2, page_tree) | 
											getBit(2 * counter2 + 1,
									page_tree),
									page_tree);
			}
		}
		//add the proper structure to the linked list
		//scan to the last structure
		for (fill_me = first_pages_range; fill_me->next != 0; fill_me = fill_me->next);
		fill_me->address = address;
		fill_me->length = actual_size;
		fill_me->next = (struct page_range *)alloc_bytes(sizeof(struct page_range));
		(fill_me->next)->previous = fill_me;	//make sure the new one points back to first one
		fill_me = fill_me->next;
		fill_me->address = 0;
		fill_me->length = 0;
		fill_me->next = 0;
		use_me = (void*)address;
		leave_spinlock(SL_MEM_MNG);
//display("Return: ");
//PrintNumber((unsigned long)use_me);
//display("\n");
		return use_me;
	}
	leave_spinlock(SL_MEM_MNG);
//display("Return: ");
//PrintNumber((unsigned long)use_me);
//display("\n");
	return use_me;
}

void kfree(void *address)
{
	enter_spinlock(SL_MEM_MNG);
	unsigned int temp, counter;
	struct page_range *fill_me;
	if ((unsigned int)address == 0)
	{
		leave_spinlock(SL_MEM_MNG);
		return;
	}
	//was less than a page allocated?
	//check to see if the address resides in one of the (<4KB) allocate pages
	for (counter = (unsigned int)page_address; counter != 0; counter = ((unsigned int*)counter)[0x3FF])
	{	//loads the address of each page used for byte allocations into counter
		if (((unsigned int)address - ((unsigned int)address % PAGE_SIZE)) == counter)
		{
//			((unsigned int *)address)[-1] &= 0x7FFFFFFF;
			*(unsigned int*)((unsigned int)address - BYTE_GRANULARITY) &= 0x7FFFFFFF;
				//just mark it as unused, don't bother combining contiguous open segments
			leave_spinlock(SL_MEM_MNG);
			return;
		}
	}
	//is it an exact page that was allocated?
	if (getBit((unsigned int)address / PAGE_SIZE, single_pages) == 1)
	{
		free_page((unsigned int)address, 
														sizeof(unsigned int) * size_tree,
														page_tree, 
														PAGE_SIZE);
		setBit((unsigned int)address / PAGE_SIZE, 
													0, 
													single_pages);
		temp = (unsigned int)address / PAGE_SIZE + size_tree * sizeof(unsigned int) * 0x4;
		for (counter = temp / 2; counter >= 1; counter /= 2)
		{
			setBit(counter, 
														getBit(2 * counter, page_tree) |
															getBit(2 * counter + 1,
														page_tree),
														page_tree);
		}
		leave_spinlock(SL_MEM_MNG);
		return;
	}
	for (fill_me = first_pages_range; fill_me->next != 0; fill_me = fill_me->next)
	{	//scan each element in the linked list of allocated addresses
		if ((unsigned int)fill_me->address == (unsigned int)address)
		{	//free that memory
			for (counter = fill_me->address; counter < (fill_me->address + PAGE_SIZE * fill_me->length - 1); counter += PAGE_SIZE)
			{	//free these pages
				free_page((unsigned int)counter,
										sizeof(unsigned int) * size_tree,
										page_tree,
										PAGE_SIZE);
			}
			if (fill_me->previous != 0)
				fill_me->previous->next = fill_me->next;
			if (fill_me->next != 0)
				fill_me->next->previous = fill_me->previous;
			leave_spinlock(SL_MEM_MNG);
			kfree(fill_me);
			return;
		}
	}
}

//called from assembly
void *memcopy(void* s1, const void* s2, size_t n)
{
	unsigned long counter;
	for (counter = 0; counter < (n / sizeof(unsigned char)); counter++)
	{
		((unsigned char*)s1)[counter] = ((unsigned char*)s2)[counter];
	}
	return s1;
}

void *memcpy(void* s1, const void* s2, size_t n)
{
	unsigned long counter;
	for (counter = 0; counter < (n / sizeof(unsigned char)); counter++)
	{
		((unsigned char*)s1)[counter] = ((unsigned char*)s2)[counter];
	}
	return s1;
}


void setup_paging(struct multiboot_info *boot_info, size_t size)
{	//necessary information, and last byte in memory used by the kernel
	page_table = (unsigned int *)(size + (PAGE_SIZE - (size % PAGE_SIZE)));
		//this is the page (4KB) where the entire PDT goes
	unsigned int table_address = (size + (PAGE_SIZE - (size % PAGE_SIZE)) + PAGE_SIZE);
	unsigned int counter;
	unsigned int counter2;
		//these two counter variables are used for looping
		//counter2 is used for the loop that loops inside another loop
	unsigned int pages = 0;
	unsigned int *memory_look;			//used to read the memory tables
		//this stores where the NEXT page table will go (first one right after PDT)
	//the page table has to be aligned on 4KB (PAGE_SIZE) bytes
	//this rounds up to the first whole page 
	//*(size) is considered to be used
	//first thing to do is to fill the page table (each entry covers 4MB)
	//bits 9,10,11 are freely usable
	//not sure what to use the PDE/PTE bits for
	//zero the PDT, so that we won't double initalize a section
	display("\tSetting up the page directory table and page table entries\n");
	for (counter = 0; counter < 0x400; counter++)
	{	//4MB * 1024 (0x400) = 4GB
		//each entry in the page_table refers to a 4MB section of memory
		page_table[counter] = 0;
	}
	if (boot_info->flags & 64)
	{
		memory_look = (unsigned int*)boot_info->mmap_addr;
		for (memory_look = (unsigned int*)boot_info->mmap_addr;
					(unsigned int)memory_look < (boot_info->mmap_addr + boot_info->mmap_length);
					memory_look += (memory_look[0]) / sizeof(unsigned int) + 1)
		{	//scan each memory range
			pdt_ptd_range(memory_look[1], memory_look[3], &table_address, memory_look[5]);
		}
		pdt_ptd_range(0, 0x100000, &table_address, 2);
		//now we can start initializing our binary tree
		//need to repeat the entire loop again, the first time we didn't have the most important piece of imformation
			//which is needed to make the entire binary tree
			//table_address will be the address of the binary tree
		page_tree = (unsigned int*)table_address;
		for (size_tree = 1; size_tree <= largest_address; size_tree *=2);
		size_tree /= PAGE_SIZE;	//the number of bits in the bottom row of the tree
		size_tree *= 2;	//the total size of the table in bits
		size_tree /= sizeof(unsigned int) * 8;
		for (counter = 0; counter < size_tree; counter++)
		{	//initialize the tree to all memory used to prevent bugs
			page_tree[counter] = 0;
		}
		//number of unsigned ints it takes to make the entire table
		memory_look = (unsigned int*)boot_info->mmap_addr;	//need to reinitialize
		for (memory_look = (unsigned int*)boot_info->mmap_addr;
				(unsigned int)memory_look < (boot_info->mmap_addr + boot_info->mmap_length);
				memory_look += (memory_look[0]) / sizeof(unsigned int) + 1)
		{	//scan each memory range (again)
			if (memory_look[5] == 1)
			{	//only mark usable/complete pages as usable
				for (counter = memory_look[1]; (counter + 0xFFF) <= (memory_look[1] + memory_look[3] - 1); counter+= PAGE_SIZE)
				{	//it's done this way, so that if memory_look[1] is on a page boundary, we won't skip a good page
					//if it doesn't start on a page boundary, then the page is "used" or unallocatable
					if ((counter % PAGE_SIZE) == 0)
					{	//the page is good
						//bit # = size/2 + address / PAGE_SIZE
						setAddress(counter / PAGE_SIZE,1, page_tree, size_tree * sizeof(unsigned int));
						pages++;
					}
					else
					{	//this page is unusable
						counter = counter - (counter % PAGE_SIZE);
						//this will bottom it to the first lower page, then it will get bumped up by 1 page
					}
				}
			}
		}
	}
	else if (boot_info->flags & 1)
	{
		pdt_ptd_range(0, boot_info->mem_lower * 0x400, &table_address, 1);
		pdt_ptd_range(0, 0x100000, &table_address, 2);
		pdt_ptd_range(0x100000, boot_info->mem_upper * 0x400, &table_address, 1);
		//PDT and PTE's have been filled
		//now initialize the binary tree for each page
		// (only go up to the greatest unused page for this table)
		//now we can start initializing our binary tree
		//need to repeat the entire loop again, the first time we didn't have the most important piece of imformation
			//which is needed to make the entire binary tree
			//table_address will be the address of the binary tree
		page_tree = (unsigned int*)table_address;
		for (size_tree = 1; size_tree <= largest_address; size_tree *=2);
		//size_tree = largest_address + PAGE_SIZE - (largest_address % PAGE_SIZE);
		//round it up to a whole page
		size_tree /= PAGE_SIZE;	//the number of bits in the bottom row of the tree
		size_tree *= 2;	//the total size of the table in bits
		size_tree /= sizeof(unsigned int) * 8;
		//number of unsigned ints it takes to make the entire table
		for (counter = 0; counter < size_tree; counter++)
		{	//initialize the tree to all memory used to prevent bugs
			page_tree[counter] = 0;
		}
		pages = 0;
		for (counter = 0; (counter + 0xFFF) <= (0x400 * boot_info->mem_lower - 1); counter += PAGE_SIZE)
		{
			setAddress(counter / PAGE_SIZE,1, page_tree, size_tree * sizeof(unsigned int));
			pages++;
		}
		for (counter = 0x100000; (counter + 0xFFF) <= (0x100000 + 0x400 * boot_info->mem_upper - 1); counter += PAGE_SIZE)
		{
			setAddress(counter / PAGE_SIZE,1, page_tree, size_tree * sizeof(unsigned int));
			pages++;
		}
	}
	//set up the single page allocation array
	single_pages = (unsigned int*)((unsigned int)page_tree + size_tree * sizeof(unsigned int));
	//size of the single_pages array
	//is size_tree / 2
	//1 means that the page has been allocated
	for (counter = 0; counter < (sizeof(unsigned int) * 2 * size_tree); counter++)
	{
		setBit(counter, 0, single_pages);
	}
	
	
	//its time to mark off already used memory segments now
	table_address = (unsigned int)single_pages + (size_tree * sizeof(unsigned int)) / 2;
	for (counter = 0x100000; counter < (table_address + (PAGE_SIZE - table_address % PAGE_SIZE) - 1); counter += PAGE_SIZE)
	{	//mark all memory used by the kernel and data structures as used
		setAddress(counter / PAGE_SIZE, 0, page_tree, size_tree * sizeof(unsigned int));
	}
	//now it is time to finish filling out the binary tree (bottom layer is complete)
	//bit n = (2n | 2n+1)
	setAddress(0, 0, page_tree, size_tree * sizeof(unsigned int));
	for (counter = size_tree * 0x8; counter >= 1; counter /= 2)
	{	//have to start on the second lowest layer
		for (counter2 = counter; counter2 < (counter * 2); counter2++)
		{
			setBit(counter2, getBit(2 * counter2, page_tree) | getBit(2 * counter2 + 1, page_tree), page_tree);
		}
	}

	page_address = (unsigned int *)kmalloc(PAGE_SIZE);	//allocate a page for byte based allocation
	
	for(counter = 0; counter < (PAGE_SIZE / BYTE_GRANULARITY); counter++)
	//clear the page in question
		page_address[counter] = 0;
	//takes 12 bits to address a page, if bit 16 is set, then that range is used
	page_address[0] = PAGE_SIZE - 2 * BYTE_GRANULARITY;	//0x8000000 means used
		//the next to last BYTE_GRANULARITY is a NULL so the search will work
		//the last BYTE_GRANULARITY is the address of the next page
		//dont' forget that the 2*BYTE_GRAN bytes are already taken in each page
	//next page (sizeof(unsigned int))
	//sizeof(unsigned int*) bytes(length (in bytes)), then length bytes
	//n
	//\- repeats over and over
	first_pages_range = (struct page_range *)kmalloc(sizeof(struct page_range));
	first_pages_range->address = 0;
	first_pages_range->length = 0;
	first_pages_range->next = 0;
	first_pages_range->previous = 0;
	//time to test allocation and deallocation
	//free_page((unsigned int)alloc_page(size_tree * 4, page_tree, PAGE_SIZE), size_tree * 4, page_tree, PAGE_SIZE);
	//kfree(kmalloc(PAGE_SIZE));	//there is a slight problem here
	//kfree(kmalloc(0x10));		//or here with the kfree function (freed the wrong page)
	display("\tEnabling paging\n");
	EnablePaging(size + (PAGE_SIZE - (size % PAGE_SIZE)));
	display("\tNumber pages available for use: ");
	PrintNumber(countPages());
	display("\n\tNumber of pages actually mapped: ");
	PrintNumber(pages);
	display("\n");
}

