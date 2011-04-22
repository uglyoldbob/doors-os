#ifndef MEMORY_H
#define MEMORY_H

struct MemoryRange 
{	//for a singly linked list of available memory addresses (this will not be used for memory allocation)
	//this will be used to help setup the page directory
	unsigned long Base;		//base address in bytes
	unsigned long Length;		//length in bytes
	MemoryRange *Next;		//the next memory range (0 if last)
};	//dont forget the semicolon

class MemoryManagement
{
public:
	MemoryManagement();
	void Create();	//this is the real initializer, i do it this way to gaurantee that it is initialized after memory data is gathered
	void Delete();	//this is called in place of the destructor (and manually as well), so that things are done in the proper order

private:
	MemoryRange *First;	//the first memory range record
	MemoryRange *CurMem;	//the current memory range record
	unsigned long SizeRam;	//the amount of RAM that must be paged (for now the limit is 4GB)
};
#endif

MemoryManagement::MemoryManagement()
{
}

void MemoryManagement::Create()
{
	//we will store all ranges displayed to the screen
	//then create the page table at the beginning of extended memory (because it could be as large as 4MB)
	//which is larger than conventional memory
	unsigned char * Video = (unsigned char *) 0xB8000;
		//create a pointer that points to the screen buffer
	int off = 320;	//two bytes for each screen character (80 * 2 * 2)
	int pos = 14;
	unsigned char temp = 0;
	unsigned char temp2 = 0;
	unsigned char temp3 = 0;
		//off + pos is where we are reading from, 
		//and the first place we are interested in looking at is the third line	
		//temp is a temporary variable used to get data from the screen
		//temp2 is a variable for getting the maximum RAM size from ususable memory addresses
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
		//so we can test for having already read the last one
	}
	//time to get the maximum size 
	//and if there are no other values to read, take last read base and add to last read length for maximum size
	SizeRam = CurMem->Base + CurMem->Length;
		//start out with something we know is going to be the smallest possible largest RAM size
	off += 160;	
	if (Video[off + pos] != ' ')
	{	//time to read the largest RAM address required
		while (Video[off + pos] != ' ')
		{	//time to read unusable memory addresses
			temp2 = 0;
			temp3 = 0;
				//so we don't have any problems with uninitialized variables
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
			off += 160;
			pos = 0;
				//so we can test for having already read the last one
			if ((temp2 + temp3) > SizeRam)
				SizeRam = temp2 + temp3;
		}
	}
	else
	{
		off + 160;
		pos = 0;
	}
	//time to create stuff for 4MB pages, and some substructuring to divide the pages into 4KB pages
	//for cottontail memory management
	//things that need to be done
		//determine how many superpages need to be created
		//determine how many pages need to be created for each superpage
		//determine how many minipages need to be created for each page
	return;
}

	unsigned long Superpage[32];
		//1 bit per superpage (are there pages available in the superpage?
	unsigned long Pages[32];
		//1 bit per page (are there mini-pages available in the page?)
	unsigned long Minipages[128];
		//1 bit per page (are there any bytes available in the mini-page?) (4096 bytes per page)
	//1024 4MB superpages
	 //1024 4KB pages
	  //1024 1B mini-pages
