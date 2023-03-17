#include "PIC.h"
#include "entrance.h"

void setupPIC()
{	//sets up the PIC and then enables interrupts
	outportb(0x11, 0x20);
	outportb(0x11, 0xA0);	//begin initialising master and slave PIC
	outportb(0x20, 0x21);	//irq	0 maps to interrupt 32
	outportb(0x28, 0xA1);	//irq 0 for slave maps to interrupt 40
	outportb(0x04, 0x21);	//slave PIC connects to IRQ 2 of master PIC
	outportb(0x02, 0xA1);
	outportb(0x01, 0x21);	//manual EOI
	outportb(0x01, 0xA1);
	outportb(0x00, 0x21);	//enable IRQ's
	EnableInts();						//enable interrupts
}

//TODO: validate the operation of these two functions
//these functions will probably need slight modification later on 
//depending on how the spinlock / thread-safeing functions change
void clearIRQ(unsigned int which)
{	//clears only IRQ's higher than which (0 - 7)
	outportb(0xFF<<(which), 0x21);
}

void enableIRQ(unsigned int which)
{	//enables all IRQ at and lower than which (0-7)
	outportb(~(0xFF<<(which)), 0x21);
}

void setupTimer(unsigned int frequency)
{	//changes the frequency of IRQ0, the timer
	//1193180 / Hz is the number to send through to port 0x40
	//0x1234DC / frequency
	outportb(0x34, 0x43);
	outportb((0x1234DC / frequency) & 0x00FF, 0x40);
	outportb((0x1234DC / frequency)>>8, 0x40);
}
