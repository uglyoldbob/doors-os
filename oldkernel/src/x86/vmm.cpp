//vmm.cpp

//contains the code to handle swapping chunks of memory to and from disk
//at first this will handle virtual memory, but will eventually be extended to 
	//handling file access as well
#include "vmm.h"
#include "disk.h"
#include "video.h"
#include "entrance.h"

//one page at a time is to be tranferred




unsigned int load_from_disk(disk* from_here, unsigned long drive_num, unsigned long sector_number, unsigned long * to_here, unsigned long size)
{	//memory at to_here should already be allocated
	//what size is transferred in one command?
	unsigned int sector_size = from_here->bytes_per_sector(drive_num);	
	//size is granulated to bytes and is the size of a sector in bytes
	if (size == 0)
		return 0;	//nothing transferred
	for (	unsigned int counter = 0;
		counter < (size / (sizeof (unsigned long *))); 
		counter += (sector_size / (sizeof(unsigned long *)))	)
	{
		if (from_here->read_sector(drive_num, sector_number, (unsigned int *)(&to_here[counter])) != 0)
			return 0;	//failure
	}
	return size;	//success
}

unsigned int load_to_disk(disk *to_here, unsigned long drive_num, unsigned long sector_number, unsigned long* from_here, unsigned long size)
{	
	unsigned int sector_size = to_here->bytes_per_sector(drive_num);
	if (size == 0)
		return 0;	//nothing transferred
	for (	unsigned int counter = 0;
		counter < (size / (sizeof (unsigned long *))); 
		counter += (sector_size / (sizeof(unsigned long *)))	)
	{
		if (to_here->write_sector(drive_num, sector_number, (unsigned int *)(&from_here[counter])) != 0)
			return 0;	//failure
	}
	return size;	//success
}

unsigned long *get_pte(unsigned long *vmem)
{
	unsigned long *ret_val = (unsigned long*)getCR3();
	ret_val = (unsigned long*)(ret_val[(unsigned long)vmem / 0x400000] & 0xFFFFF000);
	ret_val = &ret_val[((unsigned long)vmem % 0x400000)>>12];
	return ret_val;
}

int fill_pte_np(unsigned long *vmem, unsigned long lot)
{
	unsigned long *temp;
	//each page directory entry refers to 4MB
		//each page table entry refers to a page (4 KB)
	temp = get_pte(vmem);
	*temp = lot & 0xFFFFFFFE;	//make sure that the present bit is clear
								//calling code should ensure that the bottom bit is clear before calling anyways
	//time to invalidate the TLB for the memory address that was given to us
		//aparently this command is only for 486 and higher
	invlpg_asm((unsigned long)vmem);
		//need to add support code in the invalid opcode handler to detect this instruction
	return 0;
}

int fill_pte_p(unsigned long *vmem, unsigned long code)
{
	unsigned long *temp;
	//each page directory entry refers to 4MB
		//each page table entry refers to a page (4 KB)
	temp = get_pte(vmem);
	*temp = code | 0x00000001;	//make sure that the present bit is set
								//calling code should ensure that the bottom bit is set before calling anyways
	//time to invalidate the TLB for the memory address that was given to us
		//aparently this command is only for 486 and higher
	invlpg_asm((unsigned long)vmem);
		//invalidate the cache that looks at the page table entry (does nothing on a 386)
	return 0;
}

int vmem_to_swap(unsigned long *vmem)
{	//takes a page from ram and places it into a disk/drive swap spot
	
}

int swap_to_vmem(unsigned long *vmem)
{	//take a page from swap and place it into ram
	//the page table entry for the vmem address contains the lot
	unsigned int lot;
	unsigned long *temp = get_pte(vmem);
	
}
/*
-fill out page table entry and invalidate the TLB for the virtual memory address in question
	*mark as non-present and fill in the appropriate lot number 
		-(which allows location of the page among all other pages in paged memory)
*/
