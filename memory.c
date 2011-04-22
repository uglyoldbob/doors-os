//memory.c
#include "boot_info.h"
#include "memory.h"
//page granularity binary tree
//1 - indicate the presence of a free page
	//will be used to allocate >= 1 page

extern void WaitKey();
	//waits for the pause/break key to be pressed
extern void EnablePaging(unsigned long address);
extern void Delay(unsigned long);
	//delays for mmmm milliseconds
void pdtEntry(unsigned long address, unsigned long table_address);
	//fills in the entry in the PDT regarding the address (0-4GB)
	//it will point to table_address for that memory range
void tableEntry(unsigned long address);

void printPDT(unsigned long *address);
	//this prints the PDT, with the appropiate key breaks so the screen doesnt overflow

void setBit(unsigned long bit_num, int stat);
	//sets or clears the bit in the binary tree

unsigned long getBit(unsigned long bitNum);

void setAddress(unsigned long address, int stat);
	//sets or clears (depending on stat) the bit in the binary tree that corresponds to address (0-4GB)

unsigned int getAddress(unsigned long address);
	//returns the status for address

unsigned int countPages();
	//returns the number of usable pages

unsigned long *alloc_page();
	//returns the address of 1 free page
void free_page(unsigned long address);
	//frees 1 page

void pdt_ptd_range(unsigned long address, unsigned long length, unsigned long *table_address, unsigned long code);
	//fills out the PDT, and PTE for a given memory range

unsigned long *page_table;		//stores the address of the PDT, the heart of the paging system
unsigned long size_tree;			//this is the size of the binary tree (in unsigned longs)
unsigned long *page_tree;			//this is the binary tree for pages
unsigned long largest_address;//the largest address that the tree deals with

unsigned long number_pages;
	//the number of pages used for dynamic allocation
unsigned long *page_addresses;
	//page_addresses[number_pages]
	//holds the address of each page from where allocated memory will come from
unsigned long *page_bin_tree;
	//page_bin_tree[number_pages]
	//2 byte granularity for each page, used to figure out where an allocation comes from
unsigned long *address_usage;
	//address_usage[number_pages]
	//used for the free() function, because free only takes an address for arguments
//total 4 * 4 bytes = 16 bytes total




void setup_paging(struct multiboot_info *boot_info, unsigned long size)
{	//necessary information, and last byte in memory used by the kernel
	page_table = (unsigned long *)(size + (0x1000 - (size % 0x1000)));
		//this is the page (4KB) where the entire PDT goes
	unsigned long table_address = (size + (0x1000 - (size % 0x1000)) + 0x1000);
	unsigned long counter;
	unsigned long counter2;
	unsigned int pages = 0;
	unsigned long *memory_look;			//used to read the memory tables
		//this stores where the NEXT page table will go (first one right after PDT)
	//the page table has to be aligned on 4KB (0x1000) bytes
	//this rounds up to the first whole page 
	//*(size) is considered to be used
	//first thing to do is to fill the page table (each entry covers 4MB)
	//bits 9,10,11 are freely usable
	//not sure what to use the PDE/PTE bits for
	//zero the PDT, so that we won't double initalize a section
	for (counter = 0; counter < 0x400; counter++)
	{	//4MB * 1024 (0x400) = 4GB
		page_table[counter] = 0;
	}
	if (boot_info->flags & 64)
	{
		memory_look = (unsigned long*)boot_info->mmap_addr;
		for (memory_look = (unsigned long*)boot_info->mmap_addr;
					(unsigned long)memory_look < (boot_info->mmap_addr + boot_info->mmap_length);
					memory_look += (memory_look[0]) / sizeof(unsigned long) + 1)
		{	//scan each memory range
			pdt_ptd_range(memory_look[1], memory_look[3], &table_address, memory_look[5]);
		}
		pdt_ptd_range(0, 0x100000, &table_address, 2);
		//now we can start initializing our binary tree
		//need to repeat the entire loop again, the first time we didn't have the most important piece of imformation
			//which is needed to make the entire binary tree
			//table_address will be the address of the binary tree
		page_tree = (unsigned long*)table_address;
		for (size_tree = 1; size_tree <= largest_address; size_tree *=2);
		size_tree /= 0x1000;	//the number of bits in the bottom row of the tree
		size_tree *= 2;	//the total size of the table in bits
		size_tree /= sizeof(unsigned long) * 8;
		for (counter = 0; counter < size_tree; counter++)
		{	//initialize the tree to all memory used to prevent bugs
			page_tree[counter] = 0;
		}
		//number of unsigned longs it takes to make the entire table
		memory_look = (unsigned long*)boot_info->mmap_addr;	//need to reinitalize
		for (memory_look = (unsigned long*)boot_info->mmap_addr;
				(unsigned long)memory_look < (boot_info->mmap_addr + boot_info->mmap_length);
				memory_look += (memory_look[0]) / sizeof(unsigned long) + 1)
		{	//scan each memory range (again)
			if (memory_look[5] == 1)
			{	//only mark usable/complete pages as usable
				for (counter = memory_look[1]; (counter + 0xFFF) < (memory_look[1] + memory_look[3]); counter+= 0x1000)
				{	//it's done this way, so that if memory_look[1] is on a page boundary, we won't skip a good page
					//if it doesn't start on a page boundary, then the page is "used" or unallocatable
					if ((counter % 0x1000) == 0)
					{	//the page is good
						//bit # = size/2 + address / 0x1000
						setAddress(counter,1);
						pages++;
					}
					else
					{	//this page is unusable
						counter = counter - (counter % 0x1000);
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
		page_tree = (unsigned long*)table_address;
		for (size_tree = 1; size_tree <= largest_address; size_tree *=2);
		//size_tree = largest_address + 0x1000 - (largest_address % 0x1000);
		//round it up to a whole page
		size_tree /= 0x1000;	//the number of bits in the bottom row of the tree
		size_tree *= 2;	//the total size of the table in bits
		size_tree /= sizeof(unsigned long) * 8;
		//number of unsigned longs it takes to make the entire table
		for (counter = 0; counter < size_tree; counter++)
		{	//initialize the tree to all memory used to prevent bugs
			page_tree[counter] = 0;
		}
		pages = 0;
		for (counter = 0; (counter + 0xFFF) <= (0x400 * boot_info->mem_lower - 1); counter += 0x1000)
		{
			setAddress(counter,1);
			pages++;
		}
		for (counter = 0x100000; (counter + 0xFFF) <= (0x100000 + 0x400 * boot_info->mem_upper - 1); counter += 0x1000)
		{
			setAddress(counter,1);
			pages++;
		}
	}
	number_pages = 1;	//minimum number of pages is 1
	
	
//to allocate with <4KB granularity
 //number of pages that are used for <4kb granularity
	 //1,2,3, etc...
//array of the address used for each (number of page)
	//1024 possible pages, 4 bytes each
//array of pages (2 byte granularity, 2 bytes for each (address/length))
	//2 byte, 2048 spots = 4096 bytes if only 2 bytes are allocated each and every time
	//one page for each tree that is used
//array of binary trees (each tracks just one page)
	//exactly one page long (per tree)


	//its time to mark off already used memory segments now
	table_address = page_tree + size_tree * sizeof(unsigned long);
	for (counter = 0x100000; counter < (table_address + (0x1000 - table_address % 0x1000) - 1); counter += 0x1000)
	{	//mark all memory used by the kernel and data structures as used
		setAddress(counter, 0);
	}
	//now it is time to finish filling out the binary tree (bottom layer is complete)
	//bit n = (2n | 2n+1)
	for (counter = size_tree * 0x8; counter >= 1; counter /= 2)
	{	//have to start on the second lowest layer
		for (counter2 = counter; counter2 < (counter * 2); counter2++)
		{
			setBit(counter2, getBit(2 * counter2) | getBit(2 * counter2 + 1));
		}
	}

	
	//time to test allocation and deallocation
	free_page((unsigned long)alloc_page());
	printPDT(page_table);
	EnablePaging(size + (0x1000 - (size % 0x1000)));
	display("\nNumber pages available: ");
	PrintNumber(countPages());
	display("\nNumber of pages usable: ");
	PrintNumber(pages);
	display("\n");
}


void pdtEntry(unsigned long address, unsigned long table_address)
{
	page_table[address] = table_address + 3; //read/write, present, supervisor
}

void tableEntry(unsigned long address)
{
	unsigned long *table_address = (unsigned long *)(page_table[address / 0x400000] & 0xFFFFF000);
	table_address[(address / 0x1000) % 0x400] = address + 3;
}

void printPDT(unsigned long *address)
{	//this prints the PDT, with the appropiate key breaks so the screen doesnt overflow
	display("Press puase/break to show the PDT\n");
	WaitKey();
	clear_screen();
	//8 * 25
	int counter;
	int counter2;
	for (counter2 = 0; counter2 < 0x400; counter2 += 200)
	{
		for (counter = counter2; (counter < counter2 + 200) && (counter < 0x400); counter++)
		{
			PrintNumber(address[counter]);
		}
		WaitKey();
		clear_screen();
	}
}

void setBit(unsigned long bit_num, int stat)
{	//sets or clears the bit in the binary tree
	if (stat)
	{
		page_tree[bit_num /(sizeof(unsigned long) * 8)] |= 
			1<<(bit_num % (sizeof(unsigned long) * 8));
	}
	else
	{
		page_tree[bit_num /(sizeof(unsigned long) * 8)] &= 
			~(1<<(bit_num % (sizeof(unsigned long) * 8)));
	}
}

unsigned long getBit(unsigned long bitNum)
{	//retrieives bitNum from the binary tree
	unsigned int value = page_tree[bitNum /(sizeof(unsigned long) * 8)] & 1<<(bitNum % (sizeof(unsigned long) * 8));
	value = value>>(bitNum % (sizeof(unsigned long) * 8));
	return value;
}
/*
unsigned long getBit(unsigned long bitNum, unsigned long* tree_address)
{
	unsigned long value;
	value = tree_address[bitNum /(sizeof(unsigned long) * 8)] & 1<<(bitNum % (sizeof(unsigned long) * 8));
	value = value>>(bitNum % (sizeof(unsigned long) * 8));
	return value;
}
*/

void setAddress(unsigned long address, int stat)
{	//sets or clears (depending on stat) the bit in the binary tree that corresponds to address (0-4GB)
	setBit(size_tree * 0x10 + address / 0x1000, stat);
}

unsigned int getAddress(unsigned long address)
{	//returns the status for address
	return getBit(size_tree * 0x10 + address / 0x1000);
}

unsigned int countPages()
{	//returns the number of usable pages
	unsigned int pages = 0;
	unsigned int counter;
	for (counter = 0; counter < largest_address; counter += 0x1000)
	{
		if (getAddress(counter))
		{
			pages++;
		}
	}
	return pages;
}

unsigned long *alloc_page()
{	//returns the address of 1 free page
	unsigned long unused_page = 1;
	while (unused_page < (size_tree * 0x10))
	{
		if (getBit(unused_page * 2))
		{
			unused_page *= 2;
		}
		else if (getBit(unused_page * 2 + 1))
		{
			unused_page = unused_page * 2 + 1;
		}
		else
		{
			display("Error in allocate page: ");
			return 0;	//error
		}
	}
	//scan back up the table and adjust everything
	unsigned long counter;
	setBit(unused_page, 0);
	for (counter = unused_page / 2; counter >= 1; counter /= 2)
	{
		setBit(counter, getBit(2 * counter) | getBit(2 * counter + 1));
	}
	unused_page = (unused_page - (size_tree * 0x10)) * 0x1000;
	return (unsigned long *)unused_page;
}

void free_page(unsigned long address)
{	//frees 1 page and updates the binary tree
	unsigned long bit_num = size_tree * 0x10 + address / 0x1000;
	unsigned long counter;
	setAddress(address, 1);
	for (counter = bit_num / 2; counter >= 1; counter /= 2)
	{
		setBit(counter, getBit(2 * counter) | getBit(2 * counter + 1));
	}
}

void pdt_ptd_range(unsigned long address, unsigned long length, unsigned long *table_address, unsigned long code)
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
			*table_address = *table_address + 0x1000;	//ok so the next table goes on the next page
			//fill out the PageTableDirectory we just assigned
		}
	}
	for (pdt_calc = address - (address % 0x1000);
			 pdt_calc <= (address + length); pdt_calc+= 0x1000)
	{	//starts at the first page boundary at or before the memory range starts
		//goes up page by page until it reaches the end of the memory range
		//fill in page at counter
		if (pdt_calc < (address - (address % 0x1000)))
			break;	//catch any overflow bugs from address 0xFFFFE000
		tableEntry(pdt_calc);
		//determine if memory is in range
	}
	if (code == 1)
	{	//find the largest memory address that can possibly be allocated
		largest_address = address + length - 1;
	}
}


