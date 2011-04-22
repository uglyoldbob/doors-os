#include "video.h"	//this is used for all screen I/O

//globals (some of these will go away as I implement the C/C++ standard libraries)
Video vid;
//function declarations
void display(char *chr);	
	//this will be called from out ASM code
void PrintNumber(unsigned long bob);
	//this prints an unsigned long number to the screen in hexadecimal
#include "NewDel.h"
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
extern unsigned long int EnableKeyboard(void);
void EnableMultiTasking();
	//this enables multitasking, but does not create any other tasks
int DivideZero();
	//self explanatory
unsigned long RAM_Left();
	//determines how much RAM is unused
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
	asm("sti");	//this enables interrupts
	display("We have enabled protected mode with paging.\n");
//	display("Checking keyboard...\n");
//	if (!(EnableKeyboard()))
//		return 1;
//	unsigned long *a;
//	a = (unsigned long*)0x123456;
//	*a = 0;
//	a = (unsigned long *)Allocate(1);
	//*a = 0;
	//asm("int $38");
//	PrintNumber(*a);
//	display("\n");
//	ReadSector((unsigned long)a, 0, 0);
//	display("Did it work?\t");
//	PrintNumber(*a);
	display("\n");
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
	//display("\n");
	//figure out how much memory must be mapped by finding that last addressable memory location (without virtual memory)
	//if there are no other values to read (such as non-modifiable memory), take last entry and find the last byte
	SizeRam = CurMem->Base + CurMem->Length;
		//start out with something we know is going to be the smallest possible largest RAM size
	//we have our memory ranges, time to create and allocate memory structures
	//time to create our paging tables based on RamSize (in bytes)
	//create a blank page directory (everything not present) at 1MB mark
	PhyPages = SizeRam >> 12;
		//how many pages must we use to map all memory that is currently present?
//	PhyTables = (PhyPages >> 10) + ((PhyPages % 1024) > 0 ? 1 : 0);
//		//calculate the number of page tables required to map all of that stuff, including the last partial table
	VirPages = 0;
		//we will not use virtual memory yet, because we do not possess the ability for it yet (disk I/O)
		//that will be implemented later in the development process
//	VirTables = (VirPages >> 10);
//		//calculate the number of virtual memory page tables required to map all memory
//		//the last part of a table is mapped into the unused part of the last physical memory table
//this is where we will use some configuration data for (but that's later)
//prepare data to enable paging
//all pages and tables will be declared as used, then, they will be declared as their actual status
	//time to create the page directory at the 1MB mark (because conventional is not large enough to map 4GB of RAM)
	//and i want this all to be contiguous for the kernel (physically linear)
	asm("cli");
	unsigned long * Location = (unsigned long *) 0x0100000;
		//at 1MB
	unsigned long * Location2;
	unsigned long counter = 0;
	unsigned long counter2 = 0;
	PageNum = 0;
	unsigned long VirPageNum = 0;
	unsigned long Limit;
	while((counter<<10) < PhyPages)
	{	//creates page directory entries that point to page tables with page table entries
		Location[counter] = 0x100000 + (0x1000 * (counter + 1)) + 0x3;
		//point to the location of the page table
		Location2 = (unsigned long *)(0x100000 + (0x1000 * (counter + 1)));
		//this is what writes the page table entries
		Limit = PageNum + 0x400;
		if (((counter + 1)<<12) < PhyPages)
		{	//this covers the set of pages 
			for(;PageNum < Limit; PageNum++)
			{	//1, 2, 3, 4, 5, 6, 7, 8, 9, ...
				Location2[(PageNum % 0x400)] = (PageNum<<12) + 3;
			}
		}
		else
		{	//this page table is partially in RAM
			if ((PhyPages % 0x400) == 0)	//if the number of physical pages is not a multiple of 4MB
			{	//the last table is full of usable memory
				for(;PageNum < Limit; PageNum++)
				{	
					Location2[(PageNum % 0x400)] = (PageNum<<12) + 3;
				}	
			}
			else
			{	//the last table is partially fully of usable memory
				for(;(PageNum % 0x400) <= (PhyPages % 0x400); PageNum++)
				{	//process everything everything untill the last page of physical memory
					Location2[(PageNum % 1024)] = (PageNum  * 0x1000) + 3;
				}
				counter2 = PageNum;
				while (counter2 < Limit)
				{	//sets up virtual memory pages 
					//(if virtual memory is not used, then these pages will be reported as non-existant memory)
					Location2[(PageNum % 1024)] = 0;
					if (VirPageNum < VirPages)
					{
						VirPageNum++;
						PageNum++;
					}
					counter2++;
				}
			}
		}
		counter++;
	}
	if (VirPageNum <= VirPages)
	{	//only process more page directory entries if there is more vmem to enter
		while ((counter<<10) < (PhyPages + VirPages - VirPageNum))
		{	//processes pde's (all pte will be 0)
			Location[counter] = 0x100000 + (0x1000 * (counter + 1)) + 0x3;
			Location2 = (unsigned long *)(0x100000 + (0x1000 * (counter + 1)));
			Limit = PageNum + 0x400;
			for (; PageNum < Limit; PageNum++)
			{	//not present in memory ill set a page directory entry up, but not a pte
				Location2[PageNum % 0x400] = 0;
			}
			counter++;
		}
	}
	for (;(counter<<10) < 0x400;counter++)
	{	//counter will be PhyTables + VirTables when the loop starts
		Location[counter] = 0;	//this memory will never exist anywhere
	}	//these entries exist so that there will be no bugs
	//the paging table has been created
	//create the memory usage heap and the virtual memory usage heap
	//these two heaps will be the same size
	//the system uses a different set of data to determine if memory is currently paged
	//so if the heap says it is paged and the system says it aint, then it wont be paged
	//if the system says it is paged and the heap says it aint, suspect bug or non-ewxistant memory
	Heap1 = (unsigned long *)(0x101000 + (PageNum<<2) + ((((PageNum<<2) % 0x1000) > 0) ? (0x1000 - ((PageNum<<2) % 0x1000)) : 0));
	//((((PageNum<<2) % 0x1000) > 0) ? (0x1000 - ((PageNum<<2) % 0x1000)) : 0)
		//this needs to be aligned to the nearest page
	Heap2 = (unsigned long *)(0x101000 + (PageNum<<2) + ((((PageNum<<2) % 0x1000) > 0) ? (0x1000 - ((PageNum<<2) % 0x1000)) : 0) + (((PageNum<<2) % 0x1000) > 0 ? (0x1000 - ((PageNum<<2) % 0x1000)) : 0) + (PageNum>>2));
	if (((unsigned long)Heap2 % 0x1000) > 0)
	{	//needs to be bumped up to the next page
		Heap2 = (unsigned long *)((unsigned long)Heap2 + (0x1000 - ((unsigned long)Heap2 % 0x1000)));
	}
	HeaderSize = PageNum>>5;	//this should be half the size of the heap (measured in dwords)
	//scan the memory ranges after setting the heaps to default values
	//then configure in the memory pages currently being used
	//initialize the heaps (all memory is used, all of it exists in memory)
	for (counter = 0; counter < (PageNum>>4); counter++)
	{
		Heap1[counter] = 0;	//a 1 indicates that a page is not being used
		Heap2[counter] = 0;	//a 1 indicated that a page is loaded in RAM
	}
	CurMem = First;
	counter = 0;	//keeps track of the latest memory address that has been processed
	while(CurMem != 0)
	{	//scan all of the usable memory ranges
		while (counter < CurMem->Base)
		{	//set all memory from counter to CurMem->Base as used
			//the heap is already initialized to 0, nothing more needs to be done
			counter += 0x1000;
		}
		while ((counter + 0x1000) <= (CurMem->Base + CurMem->Length))
		{	//all of these need to be declared as usable
			//also ensure the entirety of the current page lies within this range
			Heap1[HeaderSize + (counter>>17)] = Heap1[HeaderSize + (counter>>17)] | 1<<(((counter>>12) % 32));
			Heap2[HeaderSize + (counter>>17)] = Heap2[HeaderSize + (counter>>17)] | 1<<(((counter>>12) % 32));
			counter += 0x1000;
		}
		CurMem = CurMem->Next;
	}
	while (counter < (PageNum<<12))
	{	//available, paged
		Heap1[HeaderSize + (counter>>17)] = Heap1[HeaderSize + (counter>>17)] | 1<<(((counter>>12) % 32));
		counter += 0x1000;
	}
	Heap1[HeaderSize] = Heap1[HeaderSize] & 0xFFFFFFF0;
	Heap2[HeaderSize] = Heap2[HeaderSize] & 0xFFFFFFF0;	//set these pages so they will stay in RAM forever or until i say otherwise
	//declare the first 4 pages as used
	//declare all pde, pte, and heaps as used and paged (so they will not be paged)
	//from 0x100000 to the end of the second heap
	//(unsigned long)(Heap2<<1) - (unsigned long)Heap1
	for (counter = 0x100000; counter < (((unsigned long)(Heap2) * 2) - (unsigned long)Heap1); counter += 0x1000)
	{	//this covers all memory used for paging and memory allocation and management
		//declare as paged, so memory will not be placed into the pagefile
		Heap2[HeaderSize + (counter>>17)] = Heap2[HeaderSize + (counter>>17)] & (0xFFFFFFFF - 1<<(((counter>>12) % 32)));
		//set the corresponding bit in heap1 to 0 with an anding function
		Heap1[HeaderSize + (counter>>17)] = Heap1[HeaderSize + (counter>>17)] & (0xFFFFFFFF - 1<<(((counter>>12) % 32)));
	}
	//(Heap1[counter>>5] & 1<<((counter % 32)))
	//now perform anding function to both fo the heaps so we can allocate and deallocate memory
	counter2 = HeaderSize<<5;	//the width of the current level in bits (stop when we reach 1)
	counter = HeaderSize<<5;	//this is the current bit pair that is being worked on
	while (counter2 > 1)
	{	//perform anding functions on all layers until we hit the top layer
		Limit = counter + counter2;
		while (counter < Limit)
		{
			//bit (unsigned long)(counter>>1) = bit counter & bit (counter + 1)
			if (((Heap1[counter>>5] & 1<<((counter % 32)))>>(counter % 32)) | ((Heap1[(counter + 1)>>5] & 1<<(((counter + 1) % 32)))>>((counter + 1) % 32)))
			{	//set to 1
				Heap1[counter>>6] = Heap1[counter>>6] | (1<<(((counter>>1) % 32)));
			}	//no need to set it to 0, it is already 0
			if ((Heap2[counter>>5] & 1<<((counter % 32))) || (Heap2[(counter + 1)>>5] & 1<<(((counter + 1) % 32))))
			{	//set to 1
				Heap2[counter>>6] = Heap2[counter>>6] | (1<<(((counter>>1) % 32)));
			}	//no need to set it to 0, it is already 0
			counter += 2;
		}
		counter2 = counter2>>1;
		counter = counter2;
	}
	display("Available RAM: ");
	PrintNumber(RAM_Left());
	display("\n");
	//for(int CurPage = 0; 		
	//kernel (900 - 11140)
	//first page (0 - 4095) used
	//second page (4096 - 8191) used
	//third page (8192 - 12287) used
	//fourth page (12288 - 16383) *used (this is the page directly after the kernel, mark as used for just-in-case)
	//at 1MB mark is the beginning of the page table information
	//this has a 4kb header, followed by a maximum of 1024 4kb data units
	//4MB + 4KB is max size (all of this will be marked as used)

	return;
}

unsigned long RAM_Left()
{
	unsigned long FreeRam = 0;
	for (unsigned long counter = 0; counter < PageNum; counter++)
	{	//check each bit in the heap and add 0x1000 if it is set to 1
		if ((Heap1[HeaderSize + (counter>>5)] & 1<<((counter % 32))) > 0)
			FreeRam += 0x1000;	
	}
	return FreeRam;
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
