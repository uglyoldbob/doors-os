//headers to include for my custom classes (some of these will not go away)
#include "video.h"
#include "memory.h"	//this contains the class for memory management (base, length) very simple

//globals (these will ebentually go away as I implement the standard libraries
Video vid;
Memory Mem;
//function declarations
extern void Beep(void);
void display(char *chr);		//this will be called from out ASM code

int main()		//this is where the C++ portion of the kernel begins
{
	
	vid.write("We have enabled protected mode.\n");
	vid.write("Detecting amount of RAM installed...\n");
	
	vid.write("Enabling paging...\n");
	//throw in a newline sequence and add the proper code to handle that
	vid.write("We will now do nothing.\n");
	return 1;
}

void display(char *chr)
{
	vid.write(chr);
}
