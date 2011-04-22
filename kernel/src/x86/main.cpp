//qemu-system-arm -M verdex -pflash flash -monitor null -nographic -m 289

//the issues I am currently facing may be issues with functions not being re-entrant / thread safe
	//and not having code to singularly execute those functions
	//TODO: research re-entrant functions and how to make a nonreentrant function do that

//TODO: find the format of the core dump that gdb uses
	//and perform core dump analysis as another method of discovering bugs
	//format is ELF
	//not sure about much else of it

//create another thread that will monitor critical kernel data
	//and raise hell when something goes wrong
	//and i will call this thread function "hellraiser"

//#define BIG_ENDIAN      0
//#define LITTLE_ENDIAN   1
//
//int TestByteOrder()
//{
//   short int word = 0x0001;
//   char *byte = (char *) &word;
//   return(byte[0] ? LITTLE_ENDIAN : BIG_ENDIAN);
//}

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
#include "vmm.h"
#include "elf.h"
#include "string.h"

#include <sys/syscalls.h>

#define CLOCKS_PER_SEC 1000	//this is the number of times our timer variable is incremented per second (real close)

unsigned long setupFloppy();	//sets up floppy information (seperate task)
extern "C" void hellraiser();
void readSerial();

extern "C" void _main();	//this initializes global objects

//called from assembly
int main(struct multiboot_info *boot_info, unsigned long size)
{	//DONE: enable paging
	//DONE: memory management
	//TODO: detect cpu, cpuid will not work on a 386, so until later cpu support is wanted, this will not be added
	//DONE: event notification
	//build floppy disk driver
	//done: complete keyboard driver (buffer for the driver, when a byte is added, post a message about it)
	//TODO: upgrade keyboard driver, allow custom mappings, conform to a standard of some sort regarding the values for keystrokes
		//UTF-8 is a likely candidate for this, ASCII can be used, but does not allow for representation of every keystroke (not easily)
		//also UTF-8 is compatible with ASCII
	//keyboard driver uses a custom format alongside with ASCII
	//DONE: spinlocks
	//enable virtual memory
	//enable multi-tasking (software / hardware mix for the moment, fully hardware multitasking is too restrictive 
		//- each task needs a seperate entry in the gdt)
		//soon will be complete software multi-tasking
	//multi-tasking partially functional
	clear_screen();
	initialize_spinlock();

	setupPIC();
	setupTimer(CLOCKS_PER_SEC);
	display("Configuring system timer for ");
	PrintNumber(CLOCKS_PER_SEC);
	display(" hertz\n");


	display("\nConfigured spinlocks data\n");
	display("PIC initialized and interrupts enabled\n");
	//set_int_handler((void *)ser_handler, 36);
	unsigned char *first_page;	//pointer to the relocated first page

	struct TSS *newtask;
	unsigned long *temporary;

	display("Configuring memory management\n");
	setup_paging(boot_info, size);

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
	
//	display("Setting up data for multi-tasking\n");

	//initialize multi-tasking and setup the first task
//	sys_tasks = (struct task*)kmalloc (sizeof (struct task));

//	first_page = (unsigned char*)kmalloc (0x1000);

//	memcopy(first_page, 0, 0x1000);

//	setup_multi_gdt();

//	init_first_task(sys_tasks);

//	display("Main: ");
//	PrintNumber((unsigned int)main);
//	display("\nsecondary_task: ");
//	PrintNumber((unsigned int)secondary_task);
//	display("\nsetupFloppy: ");
//	PrintNumber((unsigned int)setupFloppy);
//	display("\n");

	//test multi-tasking by activating another task
	//asm("cli");
//	temporary = (unsigned long*)kmalloc(0x1000);	//one page for the stack
//	newtask = (struct TSS*)kmalloc(sizeof(struct TSS));
//	newtask->esp = (unsigned long)temporary + 0xFFC;
//	newtask->cs = 0x08;
//	newtask->ds = 0x10;
//	newtask->es = 0x10;
//	newtask->fs = 0x10;
//	newtask->gs = 0x10;
//	newtask->ss = 0x10;
//	newtask->cr3 = getCR3();
//	newtask->ldt_segment_selector = 0;
//	newtask->io_map_base_address = 0;
//	newtask->debug_trap = 0;
//	newtask->eflags = 0x00000202;			//interrupt flag set, enabling interrupts for the task
//	newtask->eip = (unsigned long)secondary_task;
//	add_task_before(newtask, sys_tasks);


	//setup a new stack for the new task
//	temporary = (unsigned long*)kmalloc(0x1000);	//one page for the stack
//	newtask->esp = (unsigned long)temporary + 0xFFC;
//	newtask->eip = (unsigned long)setupFloppy;
//	add_task_before(newtask, sys_tasks);	//add another task (hopefully it will work)



	enable_multi = 0;//1;


	
	display("Initializing message delivery subsystem\n");
	init_messaging();
	display("Configuring keyboard\n");
	if (init_keyboard() == -1)
		display("Could not initialize keyboard\n");

	printf ("Characters: %c %c \n", 'a', 65);
	printf ("Decimals: %d %ld\n", 1977, 650000);
	printf ("Preceding with blanks: %10d \n", 1977);
	printf ("Preceding with zeros: %010d \n", 1977);
	printf ("Some different radixes: %d %x %o %#x %#o \n", 100, 100, 100, 100, 100);
	printf ("floats: %4.2f %+.0e %E \n", 3.1416, 3.1416, 3.1416);
	printf ("Width trick: %*d \n", 5, 10);
	printf ("%s \n", "A string");

	printf ("Test printing of characters\n");	
	printf ("TEST:%%%+5c%%\n", 'a');
	printf ("TEST:%%%-5c%%\n", 'a');
	printf ("TEST:%%% 5c%%\n", 'a');
	printf ("TEST:%%%05c%%\n", 'a');
	printf ("TEST:%%%#5c%%\n", 'a');
	printf ("TEST:%%%05c%%\n", 'a');	
	
	printf ("Test printing of strings\n");
	printf ("TEST:%%%-5.1s%%\n", "asdfghjk");
	printf ("TEST:%%%s%%\n", "asdfghjk");
	printf ("TEST:%%%s%%\n", "asdf");
	printf ("TEST:%%%-5s%%\n", "asdf");


	for (;;);
	setupFloppy();	

//	struct driveData * (*test) ();	//declaration for the pointer
									//same when used as an argument for a function
//	test = initialize;	//setup the pointer

	//when a function pointer is passed as an argument, "test" is sufficient
//	(*test)();	//call the function with the pointer

	struct message examine;	//this will be used to retrieve messages from the system buffer
	display("\nEntering message scan loop\n");
	extern unsigned int num_messages;
	while (1)
	{
		unsigned int check;
		do 
		{
			check_system_event(&check);
			Delay(100);
		} while (check == 0);
		//wait until there is an event to process
		get_system_event(&examine);

		//retrieve the message
		switch(examine.who)
		{	//process it
			case 0:	//nothing
				break;
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
	local_drive = new floppy;
	unsigned int number_fs;
	number_fs = local_drive->number_drives();
	if (number_fs != 0)
	{
		local_fs = new filesystem*[number_fs];
		fat_fs = new fat[number_fs];
		local_fs[0] = &fat_fs[0];
		local_fs[0]->mount(local_drive, 0);
	}

/*	unsigned char *buffer = new unsigned char[0x1000];
	load_from_disk(local_drive, 0, 0, (unsigned long *)buffer, 0x200);
	for (unsigned int counter = 0; counter < (0x20 / 4); 	counter++)
	{	//display contents of buffer to make sure they are accurate (present)
		PrintNumber(((unsigned long *)buffer)[counter]);
		Delay(750);
	}*/

	if (load_module("SERIAL  SO ", local_fs[0]) != 0)
	{
		display("Error Loading\n");
		for (;;);
	}
	display("Done loading\n");
	//issue a reset for increased speed in bug "actuating"
//	outportb(0xFE, 0x64);	//reboot
	while(1){};
}

