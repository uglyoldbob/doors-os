#include "video.h"	//this is used for all screen I/O

//globals (some of these will go away as I implement the C/C++ standard libraries)
Video vid;
//function declarations
void display(char *chr);	
	//this will be called from out ASM code
void PrintNumber(unsigned long bob);
	//this prints an unsigned long number to the screen in hexadecimal
void Memory_Available();
	//prints the amount on memory that is currently unused
void GetSettings();
	//this retrieves our settings from disk (requires disk I/O to be practical)
void SetupMemory();
	//this function sets up all the memory stuff (this is called right after GetSettings() is called)
extern bool EnableFloppy(void);
	//enables floppy disk functions, returns 1 for success, 0 for failure
extern void EnablePaging(void);
	//this function sets the appropiate cr registers to enable paging (protected mode must be enabled for this to work)
void EnableMultiTasking();
	//this enables multitasking, but does not create any other tasks
int DivideZero();
	//self explanatory
bool RangeConflict(unsigned long Base1, unsigned long Length1, unsigned long Base2, unsigned long Length2);
	//determines if the two ranges overlap anywhere

//headers to include for random stuff
#include "disk.h"		//for disk I/O
#include "memory.h"	//this contains globals for memory management (base, length) very simple
#include "settings.h"	//contains settings for the operating system

int main()		//this is where the C++ portion of the kernel begins
{
	EnableFloppy();
		//get settings for our OS
	SetupMemory();
		//this sets up our memory
	display("Enabling paging...\n");
	EnablePaging();
		//this is an assembly function (located in kernel.asm)
	asm("sti");
	display("We have enabled protected mode with paging.\n");
	display("We will now do nothing.\n");
	return 0;	//a non zero return signifies an error, 0 signals all is ok
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

void SetupMemory() //sets up the master paging table (required to enable paging)
{	//read and interpret the memory ranges previously displayed to the screen
	//find out how much RAM needs to be mapped
	//find out what memory cannot be used	
	//create the page table at the beginning of extended memory (1MB)
	//because it could be as large as 4MB, which is too large for conventional memory (<=640K)
	unsigned char * Video = (unsigned char *) 0xB8000;
		//create a pointer that points to the screen buffer
	int off = 160;
		//two bytes for each screen character (80 * 2 * 2)
	int pos = 14;
	unsigned char temp = 0;
	unsigned long temp2 = 0;
	unsigned long temp3 = 0;
		//off + pos is where we are reading from, 
		//and the first place we are interested in looking at is the third line	
		//temp is a temporary variable used to get data from the screen
		//temp2/3 is a variable for getting the maximum RAM size from ususable memory addresses
	First = (MemoryRange*) 0x10000;		//where the real mode stack used to be
	CurMem = (MemoryRange*) 0x10000;
	CurMem->Next = (MemoryRange*) 0;
		//setup the first block so that i can do one loop
	//loop time? (i think so to try to save memory)
	int NextYet = 0;
		//this is to determine if it is time to allocate another memory range block
	while (Video[off + pos] != 'O' && Video[off + pos] != ' ')
	{
		if (NextYet)
		{	//setup the next block for a memory range only if the first memory range has been filled out
			CurMem->Next = (MemoryRange*)(CurMem + sizeof(MemoryRange));
			CurMem = CurMem->Next;
			CurMem->Next = 0;
				//set up the next usable memory range
			CurMem->Base = 0;
			CurMem->Length = 0;
				//so we don't have any problems with uninitialized variables
		}
		pos = 14;				
			//get base address from video memory
		CurMem->Base = 0;
		CurMem->Length = 0;
		while (Video[off + pos] != ',')
		{	//read the entire base address, stop at the end of the hex display
			temp = Video[off + pos];
			if (temp > '9')
			{
				temp -= ('A' - 10);
			}
			else
			{
				temp -= '0';
			}
			CurMem->Base = (CurMem->Base << 4) + temp;
			pos += 2;
		}
		pos += 18;
			//bring us to the next variable, Length
		while (Video[off + pos] != ' ')
		{	//read the entire length, stop at the end
			temp = Video[off + pos];
			if (temp > '9')
			{
				temp -= ('A' - 10);
			}
			else
			{
				temp -= '0';
			}
			CurMem->Length = (CurMem->Length << 4) + temp;
			pos += 2;
		}
		NextYet = 1;
		off += 160;
		pos = 0;
		//display("\n*");
		//PrintNumber((unsigned long)CurMem->Base);
		//display("\t");
		//PrintNumber((unsigned long)CurMem->Length);
		//so we can test for having already read the last one
	}
	display("\n");
	//figure out how much memory must be mapped by finding that last addressable memory location (without virtual memory)
	//if there are no other values to read (such as non-modifiable memory), take last entry and find the last byte
	SizeRam = CurMem->Base + CurMem->Length;
		//start out with something we know is going to be the smallest possible largest RAM size
/*	off += 160;	
	if (Video[off + pos] != ' ')
	{	//time to read the largest RAM address required
		temp2 = 0;
		temp3 = 0;
			//so we don't have any problems with uninitialized variables
		while (Video[off + pos] != ' ')
		{	//time to read unusable memory addresses
			pos = 14;				
				//get base address from video memory
			while (Video[off + pos] != ',')
			{	//read the entire base address, stop at the end of the hex display
				temp = Video[off + pos];
				if (temp > '9')
				{
					temp -= ('A' - 10);
				}
				else
				{
					temp -= '0';
				}
				temp2 = (temp2 << 4) + temp;
				pos += 2;
			}
PrintNumber(temp2);
display(" + ");
			pos += 18;
				//bring us to the next variable, Length
			while (Video[off + pos] != ' ')
			{	//read the entire length, stop at the end
				temp = Video[off + pos];
				if (temp > '9')
				{
					temp -= ('A' - 10);
				}
				else
				{
					temp -= '0';
				}
				temp3 = (temp3 << 4) + temp;
				pos += 2;
			}
PrintNumber(temp3);
display(" = ");

			off += 160;
			pos = 0;
				//so we can test for having already read the last one
PrintNumber(temp2 + temp3);
display("; ");
			if ((temp2 + temp3) > SizeRam)
				SizeRam = temp2 + temp3;

		}
	}
	else
	{
		off + 160;
		pos = 0;
	}
	for(;;);
	display("\nRam Size: ");
	PrintNumber(SizeRam);
	display("\n");
*/
	//we have our memory ranges, time to create and allocate memory structures
	//time to create our paging tables based on RamSize (in bytes)
	//create a blank page directory (everything not present) at 1MB mark
	PhyPages = SizeRam >> 12;
		//how many pages must we use to map all memory that is currently present?
	PhyTables = (PhyPages >> 10) + ((PhyPages % 1024) > 0 ? 1 : 0);
		//calculate the number of page tables required to map all of that stuff, including the last partial table
	VirPages = 0;
		//we will not use virtual memory yet, because we do not possess the ability for it yet (disk I/O)
		//that will be implemented later in the development process
	VirTables = (VirPages >> 10);
		//calculate the number of virtual memory page tables required to map all memory
		//the last part of a table is mapped into the unused part of the last physical memory table
//this is where we will use some configuration data for (but that's later)
//prepare data to enable paging
//all pages and tables will be declared as used, then, they will be declared as their actual status
	//time to create the page directory at the 1MB mark (because conventional is not large enough to map 4GB of RAM)
	//and i want this all to be contiguous for the kernel (physically linear)
	asm("cli");
	unsigned long * Location = (unsigned long *) 0x0100000;
		//at 1MB
	unsigned long * Location2;
	unsigned long counter = 1;
	unsigned long counter2 = 0;
	unsigned long PageNum = 0;
	counter = 0;
	while(counter < PhyTables)
	{	//creates page directory entries for the page directory, and also creates the page table that the page directory entry points to
		Location[counter] = 0x100000 + (0x1000 * (counter + 1)) + 0x3;
		//and create a page table (make sure that valid memory is marked as such
		Location2 = (unsigned long *)(0x100000 + (0x1000 * (counter + 1)));
		if ((counter + 1) < PhyTables)
		{
			for(;((PageNum + 1) % 1024) > 0; PageNum++)
			{	//1, 2, 3, 4, 5, 6, 7, 8, 9, ...
				Location[(PageNum % 1024)] = (PageNum * 0x1000) + 3;
			}
		}
		else
		{
			for(;((PageNum + 1) % 1024) < ((PhyPages + 1) % 1024); PageNum++)
			{
				Location[(PageNum % 1024)] = (PageNum * 0x1000) + 3;
			}
		}
		counter++;
	}
	for (; counter < (VirTables + PhyTables); counter++)
	{
		Location[counter] = 0;
	}
	for (;counter < 1024;counter++)
	{	//counter will be PhyTables + VirTables when the loop starts
		Location[counter] = 0;	//this memory will never exist anywhere
	}	//these entries exist so that there will be no bugs
	//the paging table has been created
	//determine the required size of the heaps (for memory management)
	for(Levels = 1, Size = 1; Size < (PhyPages + VirPages); Size *=2, Levels +=1);
	Levels -= 1;	//dont want to waste memory
	HeaderSize = 1;
	HeaderSize = HeaderSize<<(Levels - 4);	//the number of bytes that the header occupies
	//add one here if table is not large enough (to allow expansion room for virtual memory)
	//the memory usage heap will go at the very top of conventional memory, wherever that is
	//the paged out heap will go right below the memory usage table, wherever that happens to be
	//first find out the top of conventional memory
	CurMem = First;
	Location = (unsigned long *) (CurMem->Base + CurMem->Length);
	Location -= (Size / 8);
	Heap1 = Location;
	Location2 = (unsigned long *) (CurMem->Base + CurMem->Length);
	Location2 -= (Size / 4);
	Heap2 = Location2;
	//first we setup the lowest heap. this holds memory address usage
	//set things to used and not used as we read the usable memory ranges
	//then apply Doors specific memory usage items
	//once this is done, the memory usage table is worthless
	CurMem = First;
	counter2 = 0;	//this is used to setup our heaps (instead of declaring another variable)
	PageNum = 0;	//this is used to set specific bits in the heap
	for(counter = 0; CurMem != 0;)
	{
		if (counter < CurMem->Base)
		{	//declare all memory starting at counter and ending at CurMem->Base as used
			//the page at counter is not available, no exceptions
			for(int Number = counter; Number < CurMem->Base; Number += 0x1000)
			{	//Number is the address of the unusable page
				counter2 = Heap1[(Number / 0x20000) + (HeaderSize / 4)];
				PageNum = Number>>12 + HeaderSize<<3;	//bit number
				PageNum = PageNum % 256;
				PageNum = PageNum>>3;
				PageNum = 2<<PageNum;
				counter2 = counter2 & (~PageNum);	//we want this bit set to 0 (used)
			}
			counter = CurMem->Base;
		}
		//this requires special calculations (a partial free segment is declared as used)
		for(int Number = counter; Number < CurMem->Base + CurMem->Length; Number += 0x1000)
		{	//we started at the base of a page or not?
			if ((counter % 0x1000) > 0)
			{	//this is not the start of a page, declare as used
				counter2 = Heap1[(Number / 0x20000) + (HeaderSize / 4)];
				PageNum = Number>>12 + HeaderSize<<3;	//bit number
				PageNum = PageNum % 256;
				PageNum = PageNum>>3;
				PageNum = 2<<PageNum;
				counter2 = counter2 & (~PageNum);	//we want this bit set to 0 (used)
				Number = (0x1000 * int(Number / 0x1000)) + 0x1000;	//move to the base of the next page
			}
			//make sure this page doesn't protrude past the edge of this memory span
			if ((CurMem->Base + CurMem->Length - Number) < 0x1000)
			{	//partially used pages are declared as fully used
				counter2 = Heap1[(Number / 0x20000) + (HeaderSize / 4)];
				PageNum = Number>>12 + HeaderSize<<3;	//bit number
				PageNum = PageNum % 256;
				PageNum = PageNum>>3;
				PageNum = 2<<PageNum;
				counter2 = counter2 & (~PageNum);	//we want this bit set to 0 (used)
			}
			else
			{	//this page is not used and can be used for anything
				counter2 = Heap1[(Number / 0x20000) + (HeaderSize / 4)];
				PageNum = Number>>12 + HeaderSize<<3;	//bit number
				PageNum = PageNum % 256;
				PageNum = PageNum>>3;
				PageNum = 2<<PageNum;
				counter2 = counter2 & (~PageNum);	//we want this bit set to 1 (not used)
				counter2 = counter2 | PageNum;
			}
		}
		counter += CurMem->Length;
		CurMem = CurMem->Next;
	}
	if (counter < SizeRam)
	{
		//this will not yet be implemented
	}
	//time to process the first half of the first heap (lots of anding functions)
	
	
	//CurPag * 0x400000 beginning
	//0x400000 = length
	
	//for(int CurPage = 0; 		
	//kernel (900 - 11140)
	//first page (0 - 4095) has free bytes
	//second page (4096 - 8191) (FULL)
	//third page (8192 - 12287) has free bytes
	//fourth page (12288 - 16383) FREE
	
	return;
}

bool RangeConflict(unsigned long Base1, unsigned long Length1, unsigned long Base2, unsigned long Length2)
{	//[1 1][2 2]   [1 [2 1] 2]   [1 [2 2] 1]
	//[12 12]
	//[2 [1 2] 1]   [2 [1 1] 2]   [2 2][1 1]
	if ((Base1 < Base2) && ((Base1 + Length1) > Base2) ||
		(Base1 == Base2) ||
		(Base2 < Base1) && ((Base2 + Length2) > Base1))
		return 1;
	return 0;	//not conflicting
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
