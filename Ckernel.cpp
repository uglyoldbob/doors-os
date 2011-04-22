//headers to include for random stuff
#include "video.h"	//this is used for all screen I/O
#include "memory.h"	//this contains globals for memory management (base, length) very simple
#include "settings.h"	//contains settings for the operating system
#include "floppy.h"	//contains functions for reading sectors from the floppy drive

//globals (some of these will go away as I implement the C/C++ standard libraries)
Video vid;
MemoryManagement Mem;
//function declarations
extern void Beep(void);		
	//does not work (i might try debugging the BIOS beeping code to get some working beep code)
void display(char *chr);	
	//this will be called from out ASM code
void PrintNumber(unsigned long bob);
	//this prints a number based on it's location and size (does not work yet)
extern unsigned long GetCr3();
	//this returns cr3
unsigned long * GetTableAddress(unsigned long place);
	//this returns the address for a page table that maps place, which should be the beginning of a page
void Memory_Available();
	//prints the amount on memory that is currently unused
void GetSettings();
	//this retrieves our settings from disk (requires disk I/O to be practical)
void SetupMemory();
	//this function sets up all the memory stuff (this is called right after GetSettings() is called)
extern void EnablePaging(void);
	//this function sets the appropiate cr registers to enable paging (protected mode must be enabled for this to work)
void EnableMultiTasking();
	//this enables multitasking, but does not create any other tasks

int main()		//this is where the C++ portion of the kernel begins
{
	GetSettings();
		//get settings for our OS
	Mem.Create();
		//this enables our memory management class
	return 1;	//means success
}

int DivideZero()
{
	int Final = 7;
	return (Final / 0);
}

void display(char *chr)
{
	vid.write(chr);
}

void Memory_Available()
{
	return;
}

unsigned long * GetTableAddress(unsigned long place)
{
	return 0;
}	

void PrintNumber(unsigned long bob)
{	//this prints a 32 bit number (8 hex digits)
	unsigned long Temp = 0;
	display("0x");
	for (int counter = 7; counter >= 0; counter--)
	{	//this is a countdown, because we write the most signifigant nibble first
		Temp = ((bob >> (counter * 4)) & 0xF);
		if (Temp > 9)
		{
			Temp += ('A' - 10);
		}
		else
		{
			Temp += '0';
		}
		vid.put((unsigned char)(Temp));
	}
}


void EnableMultiTasking()
{
//things that need to be done for this to work
//load >= 1 TSS and descriptor for it
//TSS descriptors are in the GDT
//ltr to load a segment selector for a TSS descriptor
//load segment selector before making first switch
//
}
