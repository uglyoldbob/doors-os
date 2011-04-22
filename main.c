//main.c
#include "boot_info.h"
#include "video.h"
#include "interrupt_table.h"
#include "memory.h"

#define CLOCKS_PER_SEC 1000	//this is the number of times our timer variable is incremented per second (real close)
extern unsigned long timer;

int main(struct multiboot_info *boot_info, unsigned long size)
{	//TODO: enable paging
	//memory management
	//detect cpu
	
	//build floppy disk driver
	//complete keyboard driver
	//enable virtual memory
	//enable multi-tasking
	setup_paging(boot_info, size);
	return 0;
}
