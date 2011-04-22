//vmm.h
#include "disk.h"

unsigned int load_from_disk(disk* from_here, unsigned long drive_num, 
				unsigned long sector_number, unsigned long * to_here, unsigned long size);
//returns number of bytes transferred

unsigned int load_to_disk(disk *to_here, unsigned long drive_num, 
				unsigned long sector_number, unsigned long* from_here, unsigned long size);
//returns the number of bytes transferred

//for the time being, the memory map will not be expanded in order to accomodate memory use expansion

//these functions properly set up information in the page table entry associated with the the given page
int fill_pte_np(unsigned long *vmem, unsigned long lot);
int fill_pte_p(unsigned long *vmem, unsigned long code);

//virtual memory information
/*
device name
virtual memory address

*/

struct virtual_mem
{
	char *device_name;
	char *file_name;	//might be a virtual memory partition
						//i need to read into virtual memory partitions
	unsigned long *v_address;	//virtual address
	unsigned long *offset;		//offset into the file or partition
};
