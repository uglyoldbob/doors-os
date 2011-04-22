//main.c
#include "boot_info.h"
#include "video.h"
#include "interrupt_table.h"
#include "memory.h"
#include "floppy.h"
#include "PIC.h"
#include "keyboard.h"
#include "spinlock.h"
#include "message.h"
#include "disk.h"
#include "fat.h"
#include "tss.h"
#include "entrance.h"

#include <sys/syscalls.h>

#define CLOCKS_PER_SEC 1000	//this is the number of times our timer variable is incremented per second (real close)

unsigned long setupFloppy();	//sets up floppy information (seperate task)



char *key_show[] = {
"NULL",
"Esc ",
"1! ",
"2@ ",
"3# ",
"4$ ",
"5% ",
"6^ ",
"7& ",
"8* ",
"9( ",
"0) ",
"-_ ",
"=+ ",
"BK ",
"TB ",
"qQ ",
"wW ",
"eE ",
"rR ",
"tT ",
"yY ",
"uU ",
"iI ",
"oO ",
"pP ",
"[{ ",
"]} ",
"EN ",
"LC ",
"aA ",
"sS ",
"dD ",
"fF ",
"gG ",
"hH ",
"jJ ",
"kK ",
"lL ",
";: ",
"'\" ",
"`~ ",
"LS ",
"\\| ",
"zZ ",
"xX ",
"cC ",
"vV ",
"bB ",
"nN ",
"mM ",
",< ",
".> ",
"/? ",
"RS ",
"* ",
"LA ",
"' ' ",
"CAP ",
"F1 ",
"F2 ",
"F3 ",
"F4 ",
"F5 ",
"F6 ",
"F7 ",
"F8 ",
"F9 ",
"F10 ",
"NUM ",
"SCR ",
"HO7 ",
"UP8 ",
"PU9 ",
"- ",
"LE4 ",
"5 ",
"RI6 ",
"+ ",
"END1 ",
"DWN2 ",
"PD3 ",
"INS0 ",
"DEL. ",
"RCTRL ",	
"/ ",	
"PrtScr ",	
"F11 ",
"F12 ",
"RA ",
"ENT ",
"HME ",
"UP ",
"PGUP ",
"LFT ",
"RGT ",
"END ",
"DWN ",
"PDWN ",
"INSE ",
"DEL ",
"LWIN ",
"RWIN ",
"MENU ",
"PAUSE "
};

//called from assembly
int main(struct multiboot_info *boot_info, unsigned long size)
{	//DONE: enable paging
	//DONE: memory management
	//TODO: detect cpu
	//DONE: event notification
	//build floppy disk driver
	//done: complete keyboard driver (buffer for the driver, when a byte is added, post a message about it)
	//TODO: upgrade keyboard driver, allow custom mappings, conform to a standard of some sort regarding the values for keystrokes
		//UTF-8 is a likely candidate for this, ASCII can be used, but does not allow for representation of every keystroke (not easily)
		//also UTF-8 is compatible with ASCII
	//DONE: spinlocks
	//enable virtual memory
	//enable multi-tasking (software / hardware mix for the moment, fully hardware multitasking is too restrictive 
		//- each task needs a seperate entry in the gdt)
		//soon will be complete software multi-tasking
	//multi-tasking functional
	clear_screen();
	initialize_spinlock();
	display("\nConfigured spinlocks data\n");
	display("PIC initialized and interrupts enabled\n");
	setupPIC();
	PrintNumber(getEIP());
	//unsigned char drive;	//stores information about the drive Doors was loaded from
	unsigned long counter;
	unsigned char *first_page;	//pointer to the relocated first page

	struct TSS *newtask;
	unsigned long *temporary;

	//enter_spinlock(SL_MEM_MNG);
	//leave_spinlock(SL_MEM_MNG);

	display("Configuring system timer for ");
	PrintNumber(CLOCKS_PER_SEC);
	display(" hertz\n");
	setupTimer(CLOCKS_PER_SEC);

	display("Configuring memory management\n");
	setup_paging(boot_info, size);


	if (boot_info->flags & 0x2)
	{	//0x00xxxxxx = floppy, 0xE0xxxxxx = CD, 0x80xxxxxx = hard drive
		display("Boot device: ");
		PrintNumber(boot_info->boot_device);
		display("\n");
	}
	if (boot_info->flags & 0x4)
	{	//check for a commandline given to the kernel
		display("Command line: ");
		display((char*)boot_info->cmdline);
		display("\n");
	}
	
	display("Setting up data for multi-tasking\n");

	//initialize multi-tasking and setup the first task
	sys_tasks = malloc (sizeof (struct task));
	first_page = malloc (0x1000);
	memcopy(first_page, 0, 0x1000);
	setup_multi_gdt();
	init_first_task(sys_tasks);

	display("Main: ");
	PrintNumber(main);
	display("\nsecondary_task: ");
	PrintNumber(secondary_task);
	display("\nsetupFloppy: ");
	PrintNumber(setupFloppy);
	display("\n");

	//test multi-tasking by activating another task
	//asm("cli");
	temporary = malloc(0x1000);	//one page for the stack
	newtask = malloc(sizeof(struct TSS));
	newtask->esp = (unsigned long)temporary + 0xFFC;
	newtask->cs = 0x08;
	newtask->ds = 0x10;
	newtask->es = 0x10;
	newtask->fs = 0x10;
	newtask->gs = 0x10;
	newtask->ss = 0x10;
	newtask->cr3 = getCR3();
	newtask->ldt_segment_selector = 0;
	newtask->eflags = 0x00000202;			//interrupt flag set, enabling interrupts for the task
	newtask->eip = (unsigned long)secondary_task;
	add_task_before(newtask, sys_tasks);


	//setup a new stack for the new task	
	temporary = malloc(0x1000);	//one page for the stack
	newtask->esp = (unsigned long)temporary + 0xFFC;
	newtask->eip = (unsigned long)setupFloppy;
	add_task_before(newtask, sys_tasks);	//add another task (hoepfully it will work)
	//display("\nReturned to main ");
	//PrintNumber(getEIP());
	//display("\n");
	enable_multi = 1;
	//asm("sti");
	
//	display("sys_tasks = ");
//	PrintNumber(sys_tasks);
//	display("\n");
//	sys_tasks = next_state(1, sys_tasks, malloc(0x1000));
//	display("sys_tasks = ");
//	PrintNumber(sys_tasks);
//	display("\n");

	display("Initializing message delivery subsystem\n");
	init_messaging();
//	display("Checking for drives connected to IDE controllers.\n");
//	examine_ide();
	display("Configuring keyboard\n");
	init_keyboard();

//	struct driveData * (*test) ();	//declaration for the pointer
									//same when used as an argument for a function
//	test = initialize;	//setup the pointer

	//when a function pointer is passed as an argument, "test" is sufficient
//	(*test)();	//call the function with the pointer

	//enter_spinlock(SL_MEM_MNG);
	struct message examine;	//this will be used to retrieve messages from the system buffer

	display("\nEntering message scan loop\n");
	while (1)
	{
		unsigned int check;
		do 
		{
			check_system_event(&check);
		} while (check == 0);
		//wait until there is an event to process
		get_system_event(&examine);
		//retrieve the message
		switch(examine.who)
		{	//process it
			case KEYBOARD:
				if ((examine.data1 & MAKE) == MAKE)
				{
					switch (examine.data1 & 0xFF)
					{	//here is where specific actions for keyboard buttons will be handled
						case KEY_ESCAPE:
							outportb(0xFE, 0x64);	//reboot
							display("\nThe escape key has been pressed\n");
							break;
						default:
							display((char*)(key_show[examine.data1 & 0xFF]));
							break;
					}
				}
					break;
			default:
				display("Unknown source of message");
				PrintNumber(examine.who);
				display("\n");
				break;
		}
	}
}

unsigned long setupFloppy()
{	//seperate task to initialize/detect the floppy drives
	struct FatBootSector floppya;
	unsigned char *storage;
	display("Looking for floppy drives\n");
	initialize();
	storage = malloc(0x1000);
	display("Reading a sector from the floppy drive:\n");
	//if (floppy_read_sector(1, 0, storage, 0x3F0) == -1)
	//	display("Error reading sector from floppy drive\n");
	unsigned char *OemName;
	OemName = malloc(9);
	OemName[0] = 0;

	struct sectorReturn floppy_boot;
	floppy_boot = readSector(0, 0);
	if (floppy_boot.size == 0)
	{
		display("Error reading sector from floppy drive\n");
	}
	else
	{
		load_boot_sector(0, &floppya, floppy_boot.data);
	}

/*	display("Information as follows:\n");
	for (counter = 0; counter < sector_size / sizeof(unsigned long); counter++)
		PrintNumber(((unsigned long*)storage)[counter]);
	display("\n");*/
	
	memcopy(OemName, &floppy_boot.data[3], 8);
	display("The Disk is named: ");
	display(OemName);
	display("\tBytes per sector:");
	PrintNumber(floppy_boot.data[12] * 0x100 + floppy_boot.data[11]);
	display("\nSectors per cluster:");
	PrintNumber(floppy_boot.data[13]);
	display("\tNumber of reserved sectors:");
	PrintNumber(floppy_boot.data[15] * 0x100 + floppy_boot.data[14]);
	display("\nNumber of FAT tables:");
	PrintNumber(floppy_boot.data[16]);
	display("\tEntries in the root directory:");
	PrintNumber(floppy_boot.data[18] * 0x100 + floppy_boot.data[17]);
	display("\nTotal number of sectors:");
	PrintNumber(floppy_boot.data[20] * 0x100 + floppy_boot.data[19]);
	display("\tFat size in sectors:");
	PrintNumber(floppy_boot.data[23] * 0x100 + floppy_boot.data[22]);
	display("\n");

	while(1){};
}
