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

#define CLOCKS_PER_SEC 1000	//this is the number of times our timer variable is incremented per second (real close)
extern unsigned int timer;		//entrance.asm

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

int main(struct multiboot_info *boot_info, unsigned long size)
{	//DONE: enable paging
	//DONE: memory management
	//TODO: detect cpu
	//DONE: event notification
	//build floppy disk driver
	//done: complete keyboard driver (buffer for the driver, when a byte is added, post a message about it)
	//enable virtual memory
	//enable multi-tasking
	unsigned char drive;	//stores information about the drive Doors was loaded from
	unsigned long counter;
	clear_screen();
	if (boot_info->flags & 0x2)
	{	//0x00xxxxxxxx = floppy, 0xE0xxxxxxxx = CD
		display("Boot device: ");
		PrintNumber(boot_info->boot_device);
		display("\n");
	}
	if (boot_info->flags & 0x4)
	{	//check for a commandline given to the kernel
		display((char*)boot_info->cmdline);
		display("\n");
	}
	display("Initializing spinlock data\n");
	initialize_spinlock();
	display("Configuring memory management\n");
	setup_paging(boot_info, size);
	display("Initializing message delivery subsystem\n");
	init_messaging();
	display("Configuring system timer for ");
	PrintNumber(CLOCKS_PER_SEC);
	display(" hertz\n");
	setupTimer(CLOCKS_PER_SEC);
	display("Setting up PIC and enabling interrupts\n");
	setupPIC();
	display("Looking for floppy drives\n");
	initialize_floppy();
	display("Checking for drives connected to IDE controllers.\n");
	examine_ide();
	display("Configuring keyboard\n");
	init_keyboard();
	unsigned char *storage;
	unsigned char *OemName;
	OemName = malloc(9);
	OemName[0] = 0;
	storage = malloc(storage);
	display("Reading a sector from the floppy drive:\n");
	if (read_sector(0, 0, storage, FLOPPY_PRIMARY_BASE) == -1)
		display("Error reading sector from floppy drive\n");
	display("First sector information is loaded at:");
	PrintNumber(sector_size);
	display("\n");
//	display("Information as follows:\n");
//	for (counter = 0; counter < sector_size / sizeof(unsigned long); counter++)
//		PrintNumber(((unsigned long*)storage)[counter]);
//	display("\n");
	memcopy(OemName, &storage[3], 8);
	display("The Disk is named: ");
	display(OemName);
	display("\nBytes per sector:");
	PrintNumber(storage[12] * 0x100 + storage[11]);
	display("\nSectors per cluster:");
	PrintNumber(storage[13]);
	display("\nNumber of reserved sectors:");
	PrintNumber(storage[15] * 0x100 + storage[14]);
	display("\nNumber of FAT tables:");
	PrintNumber(storage[16]);
	display("\nEntries in the root directory:");
	PrintNumber(storage[18] * 0x100 + storage[17]);
	display("\nTotal number of sectors:");
	PrintNumber(storage[20] * 0x100 + storage[19]);
	display("\nThis is negative 1:");
	PrintNumber(-1);
	display("\n");
	//enter_spinlock(SL_MEM_MNG);
	struct message examine;	//this will be used to retrieve messages from the system buffer
	display("\nEntering message scan loop\n");
	while (1)
	{
		unsigned int check;
		do {check_system_event(&check);} while (check == 0);
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
	return 0;
}
