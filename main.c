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
	//TODO: event notification
	//build floppy disk driver
	//complete keyboard driver (buffer for the driver, when a byte is added, post a message about it)
	//enable virtual memory
	//enable multi-tasking
	display("Initializing spinlock data\n");
	initialize_spinlock();
	display("Configuring memory management\n");
	setup_paging(boot_info, size);
	display("Initializing message delivery subsystem\n");
	init_messaging();
	display("Setting up PIC and enabling interrupts\n");
	setupPIC();
	display("Configuring system timer for ");
	PrintNumber(CLOCKS_PER_SEC);
	display(" hertz\n");
	setupTimer(CLOCKS_PER_SEC);

	display("Configuring keyboard\n");
	init_keyboard();
//	display("Initializing floppy drive\n");
//	initialize_floppy(FLOPPY_PRIMARY_BASE);
	//initialize/test the floppy drive
	//enter_spinlock(SL_MEM_MNG);
	struct message examine;	//this will be used to retrieve messages from the system buffer
	display("Entering message scan loop\n");
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
					display((char*)(key_show[examine.data1 & 0xFF]));
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
