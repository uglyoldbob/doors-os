//NewDel.h
//defines the things necessary to get new and delete working
//I hope this operator overloading works
void *operator new (unsigned long length)
//void *__builtin_new(unsigned long length)
{	//for now, if memory is full, this function will loop until it becomes available
	//scan the first heap, looking for available memory
	//then make sure that is in RAM before returning the address to it
	display("Allocate length:\t");
	PrintNumber(length);
	display("\n");
	return (void*)0;
}
void *operator new[] (unsigned long length)
//void *__builtin_new[](unsigned long length)
{	//for now, if memory is full, this function will loop until it becomes available
	//scan the first heap, looking for available memory
	//then make sure that is in RAM before returning the address to it
	display("Allocate length[]:\t");
	PrintNumber(length);
	display("\n");
	return (void*)0;
}

void operator delete (void* address)
{
	display("Deallocate address:\t");
	PrintNumber((unsigned long)address);
	display("\n");
}

void operator delete[] (void* address)
{
	display("Deallocate address[]:\t");
	PrintNumber((unsigned long)address);
	display("\n");
}
