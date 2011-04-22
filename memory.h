#ifndef MEMORY_H
#define MEMORY_H
//define a few globals used for memory management in the kernel

struct MemoryRange 
{	//for a singly linked list of available memory addresses (this will not be used for memory allocation)
	//this will be used to help setup the page directory
	unsigned long Base;		//base address in bytes
	unsigned long Length;		//length in bytes
	MemoryRange *Next;		//the next memory range (0 if last)
};	//dont forget the semicolon

	MemoryRange *First;	//the first memory range record
	MemoryRange *CurMem;	//the current memory range record
	unsigned long SizeRam;	//the amount of RAM that must be paged (for now the limit is 4GB)
	unsigned long PhyPages;	//the number of pages for PHYSICAL RAM
//	unsigned long PhyTables;//self explanatory
	unsigned long VirPages;	//the number of pages that are not outside of the range of RAM (virtual memory)
//	unsigned long VirTables;//self explanatory
	unsigned long PageNum;	//the number of pages currently being mapped
//	unsigned long End;		//memory after the second heap
	unsigned long HeaderSize;	//2^(n-1) (half the size of the heap)
	unsigned long *Heap1;	//tracks memory usage
							//1 = memory segment is not currently used (you can use it for anything)
	unsigned long *Heap2;	//tracks paging status (0 = paged)
							//the computer has a seperate list for this
							//if a segment is not paged and its bit is not set
							//then it will not be paged
//i will have some underlying allocate / deallocate functions
//allocate and deallocate will be called from an interrupt handler
//called from interrupt 32
void *Allocate(unsigned long howmanypages)
{	//looks for howmanypages consecutive available pages
	//if not enough pages are found
	//i will eventually make this available to other processes via an interrupt
	unsigned long bitNum;	//bit number of the heap
	unsigned long Return;	//the address that we allocated (only returned if it worked)
	unsigned long Temp;	//used to store temporary results
	if ((Heap1[0] & 0x2) == 0)
	{	//no memory available
		return 0;
	}
	if (howmanypages == 1)
	{	//check the second bit to make sure there is availabe memory
		bitNum = 1;
		do
		{	//loop until we hit the bottom of the heap
			//n = 2n
			//check bit n, if that is bad, go to n + 1
			bitNum = bitNum<<1;
			if ((Heap1[bitNum>>5] & 1<<(bitNum % 32)) > 0)
			{
			}
			else
			{	//check the second bit (for real)
				bitNum++;
				if ((Heap1[bitNum>>5] & 1<<(bitNum % 32)) > 0)
				{
				}
				else
				{
					return 0;
				}
			}
		} while (bitNum <= (HeaderSize<<5));
		//if it makes it here, we have to manually check the last leyer for some reason
		Return = (bitNum - (HeaderSize<<5)) * 0x1000;
		//ok, now declare the page we just found as used
		Heap1[bitNum>>5] = Heap1[bitNum>>5] & (0xFFFFFFFF - (1<<((bitNum % 32))));
		//perform anding functions up to the top layer
		//we need two cases (bitNum % 2) == 0 and (bitNum % 2) == 1
		while(bitNum > 1)
		{	//bitNum, bitNum + 1
			if ((bitNum % 2) == 0)
			{	//bitNum, bitNum + 1
				if (0 == (((Heap1[bitNum>>5] & 1<<((bitNum % 32)))>>(bitNum % 32)) | ((Heap1[(bitNum + 1)>>5] & 1<<(((bitNum + 1) % 32)))>>((bitNum + 1) % 32))))
				{	//set to 0
					Heap1[bitNum>>6] = Heap1[bitNum>>6] & (0xFFFFFFFF - (1<<(((bitNum >>1)% 32))));
				}
			}
			else
			{	//bitNum -1, bitNum
				if (0 == (((Heap1[bitNum>>5] & 1<<((bitNum % 32)))>>(bitNum % 32)) | ((Heap1[(bitNum - 1)>>5] & 1<<(((bitNum - 1) % 32)))>>((bitNum - 1) % 32))))
				{	//set to 0
					Heap1[bitNum>>6] = Heap1[bitNum>>6] & (0xFFFFFFFF - (1<<(((bitNum>>1) % 32))));
				}
			}
			bitNum = bitNum>>1;
		}
	}
	else
	{	//this will still be hard to do
		//create a subnet with howmanypages bits set to 1
		//this form should only be used when physical memory needs to be contiguous (takes much longer)
		//1. find an open page (or the next open page)
		//2. count the number of open pages after this page
		//3. stop when we reach howmanypages, if not enough, go to step 1
		bitNum = 0;
		Return = 0;
//		display("");	//this won't work without this for some reason (sometimes)
		while (Temp < howmanypages)
		{
			if ((Heap1[HeaderSize + (bitNum>>5)] & 1<<((bitNum % 32))) > 0)
			{	//this page is available
				Temp++;
				if (Return == 0)
					Return = bitNum * 0x1000;
			}
			else
			{
				Return = 0;
				Temp = 0;
			}
			bitNum++;
			if (bitNum > (HeaderSize<<5))
				return 0;
		}
		//now that we have discovered some memory that we can allocate, declare it as used
		//and update the heap
		bitNum = Return>>12;
		Temp = bitNum + howmanypages;
		while (bitNum < Temp)
		{
			Heap1[HeaderSize + (bitNum>>5)] = Heap1[HeaderSize + (bitNum>>5)] & (0xFFFFFFFF - (1<<((bitNum % 32))));
			bitNum++;
		}
		//ok now we need to update the rest of the heap
		unsigned long counter2 = HeaderSize<<5;	//the width of the current level in bits (stop when we reach 1)
		unsigned long counter = HeaderSize<<5;	//this is the current bit pair that is being worked on
		unsigned long Limit;
		while (counter2 > 1)
		{	//perform anding functions on all layers until we hit the top layer
			Limit = counter + counter2;
			while (counter < Limit)
			{
				//bit (unsigned long)(counter>>1) = bit counter & bit (counter + 1)
				if (((Heap1[counter>>5] & 1<<((counter % 32)))>>(counter % 32)) | ((Heap1[(counter + 1)>>5] & 1<<(((counter + 1) % 32)))>>((counter + 1) % 32)))
				{	//set to 1
					Heap1[counter>>6] = Heap1[counter>>6] | (1<<(((counter>>1) % 32)));
				}
				else
				{	//set to 0
					Heap1[counter>>6] = Heap1[counter>>6] & (0xFFFFFFFF - (1<<(((counter>>1) % 32))));
				}
				counter += 2;
			}
			counter2 = counter2>>1;
			counter = counter2;
		}
	}
	return (void *)Return;
}

void Deallocate(void *address, unsigned long length)
{	//this function is easy
	//declare pages as usable
	unsigned long bitNum = (unsigned long)address>>12;
	unsigned long Temp = bitNum + length;
	while (bitNum < Temp)
	{
		Heap1[HeaderSize + (bitNum>>5)] = Heap1[HeaderSize + (bitNum>>5)] | (1<<((bitNum % 32)));
		bitNum++;
	}
	//update the heap to reflect the changes
	unsigned long counter2 = HeaderSize<<5;	//the width of the current level in bits (stop when we reach 1)
	unsigned long counter = HeaderSize<<5;	//this is the current bit pair that is being worked on
	unsigned long Limit;
	while (counter2 > 1)
	{	//perform anding functions on all layers until we hit the top layer
		Limit = counter + counter2;
		while (counter < Limit)
		{
			//bit (unsigned long)(counter>>1) = bit counter & bit (counter + 1)
			if (((Heap1[counter>>5] & 1<<((counter % 32)))>>(counter % 32)) | ((Heap1[(counter + 1)>>5] & 1<<(((counter + 1) % 32)))>>((counter + 1) % 32)))
			{	//set to 1
				Heap1[counter>>6] = Heap1[counter>>6] | (1<<(((counter>>1) % 32)));
			}
			else
			{	//set to 0
				Heap1[counter>>6] = Heap1[counter>>6] & (0xFFFFFFFF - (1<<(((counter>>1) % 32))));
			}
			counter += 2;
		}
		counter2 = counter2>>1;
		counter = counter2;
	}
}

/*void Delete (void* address, unsigned long length)
//void __builtin_delete[](void *address, unsigned long length)
{
	display("Deallocate length:\t");
	PrintNumber(length);
	display("\n");
}*/
#endif
