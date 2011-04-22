#ifndef MEMORY_H
#define MEMORY_H
struct MemoryRange 
{	//for a singly linked list of available memory addresses
	long Base;			//base address in bytes
	long Length;		//length in bytes
	MemoryRange *Next;	//the next memory range (0 if last)
};	//dont forget the semicolon

class Memory
{
public:
	Memory();
	~Memory();
private:
	int bob;
};
#endif
