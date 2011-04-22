#include "spinlock.h"


//exact breakpoints are enabled by default
//detection of access to debug registers

int enableBreakpoint(int point, int l_or_g, unsigned long address, int code, int length)
{
	//specify a certain breakpoint register to use, or specify 0 for choose one
	//local or global breakpoint (in order for local to work properly it should probably be called from the task that wants it
	//address
	//code - two bit code for what to break on (386/486 does not support I/O debugging this way)
	//length - two bit code for data length (1,2,4 bytes)
	//return value: breakpoint used (0 = no breakpoint)
	switch(point)
	{
		case 0:		//pick an open breakpoint
			break;
		case 1:
			return 1;
			break;
		case 2:
			return 2;
			break;
		case 3:
			return 3;
			break;
		case 4:
			return 4;
			break;
		default:	//invalid breakpoint specifier
			return 0;
			break;
	}
}

void clearBreakpoint(int point)
{	//clears/disables a specified breakpoint
	//disable the L/G flags for that break
	switch(point)
	{
		case 1:
			asm("pushl %eax");
			asm("movl %dr7, %eax");
			asm("andl $0xFFFFFFFC, %eax");
			asm("movl %eax, %dr7");
			asm("popl %eax");
			break;
		case 2:
			asm("pushl %eax");
			asm("movl %dr7, %eax");
			asm("andl $0xFFFFFFF3, %eax");
			asm("movl %eax, %dr7");
			asm("popl %eax");
			break;
		case 3:
			asm("pushl %eax");
			asm("movl %dr7, %eax");
			asm("andl $0xFFFFFFCF, %eax");
			asm("movl %eax, %dr7");
			asm("popl %eax");
			break;
		case 4:
			asm("pushl %eax");
			asm("movl %dr7, %eax");
			asm("andl $0xFFFFFF3F, %eax");
			asm("movl %eax, %dr7");
			asm("popl %eax");
			break;
		default:	//invalid breakpoint specifier
			return;
			break;
	}

}

void hellraiser()
{	//this function raises hell when something goes wrong
	//this will be a debugger function thread

	display("\nHellRaiser to the rescue!\n");
	clearBreakpoint(1);
	clearBreakpoint(2);
	clearBreakpoint(3);
	clearBreakpoint(4);
	while (1){};
	enableBreakpoint(1, 3, enter_spinlock, 0, 0);
}

//debug exception
//breakpoint exception
//resume and trap flags (eflags)
//trap flags (tss)

