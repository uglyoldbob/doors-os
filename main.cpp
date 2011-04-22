//main.c
//gdb -s kernel.bin -ex 'target remote :1234'
//gdbstub: enabled=1, port=1234, text_base=0x100000, data_base=0x100000, bss_base=0x100000
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
#include "serial.h"

#include <sys/syscalls.h>

#define CLOCKS_PER_SEC 1000	//this is the number of times our timer variable is incremented per second (real close)

unsigned long setupFloppy();	//sets up floppy information (seperate task)
void readSerial();

extern "C" void _main();	//this initializes global objects
extern memory global_memory;

const char *key_show[] = {
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
	//multi-tasking partially functional
	clear_screen();
	initialize_spinlock();
	display("\nConfigured spinlocks data\n");
	display("PIC initialized and interrupts enabled\n");
	setupPIC();
	set_int_handler((void *)ser_handler, 36);
	//unsigned char drive;	//stores information about the drive Doors was loaded from
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
	global_memory.setup_paging(boot_info, size);

	_main();	//^- because some global objects might use new/delete in their constructors

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
	sys_tasks = (struct task*)malloc (sizeof (struct task));
	first_page = (unsigned char*)malloc (0x1000);
	memcopy(first_page, 0, 0x1000);
	setup_multi_gdt();
	init_first_task(sys_tasks);

	display("Main: ");
	PrintNumber((unsigned int)main);
	display("\nsecondary_task: ");
	PrintNumber((unsigned int)secondary_task);
	display("\nsetupFloppy: ");
	PrintNumber((unsigned int)setupFloppy);
	display("\n");

	//test multi-tasking by activating another task
	//asm("cli");
	temporary = (unsigned long*)malloc(0x1000);	//one page for the stack
	newtask = (struct TSS*)malloc(sizeof(struct TSS));
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
	temporary = (unsigned long*)malloc(0x1000);	//one page for the stack
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
	if (init_keyboard() == -1)
		display("Could not initialize keyboard\n");

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
				if ((examine.data1 & MAKE) > 0)
				{
					switch (examine.data1 & 0xFF)
					{	//here is where specific actions for keyboard buttons will be handled
						case KEY_ESCAPE:
							display("\nReboot has started\n");
							outportb(0xFE, 0x64);	//reboot
							break;
						default:
						{
							if ((examine.data1 & MULTI) == 0)
							{	//single byte
								put((char)(examine.data2 & 0xFF));
							}
							else
							{	//multi-byte
								display("Multibyte.");
							}
							//display((char*)(key_show[examine.data1 & 0xFF]));
							break;
						}
					}
				}
					break;
			case SERIAL:
			{
				switch (examine.data1 & 0xFF)
				{
					case 0x1B:
						display("\nReboot has started\n");
						outportb(0xFE, 0x64);	//reboot
						break;
					default:
						put((char)examine.data1);
						break;
				}
				break;
			}
			default:
				display("Unknown source of message");
				PrintNumber(examine.who);
				display("\n");
				break;
		}
	}
	display("Exiting from main()\n");
}

unsigned long setupFloppy()
{	//seperate task to initialize/detect the floppy drives
	disk *local_drive;
	filesystem **local_fs;
	fat *fat_fs;
	//local_fs = new fat;
	local_drive = new floppy;
	unsigned int number_fs;
	number_fs = local_drive->number_drives();
	if (number_fs != 0)
	{
		local_fs = new filesystem*[number_fs];
		fat_fs = new fat[number_fs];
		local_fs[0] = &fat_fs[0];
		local_fs[1] = &fat_fs[1];
		local_fs[0]->mount(local_drive, 0);
		local_fs[1]->mount(local_drive, 1);
	}
//	second_fs[0].mount(local_drive, 1);
	while(1){};
}
