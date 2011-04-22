#include "spinlock.h"
#include "memory.h"
#include "entrance.h"
/*
the spinlock states are discrete levels
a spinlock of level 5 will prevent entering spinlock level 4
also any interrupts or IRQ's that can interfere (or cause a deadlock) are disabled when the spinlock is entered
they are re-enabled when the spinlock is exited
*/



struct SL_STATES spinlock_states[NUMBER_TYPES];

void enter_spinlock(unsigned int which)
{	//enters the requested spinlock level
	//return;
	//only if the rules allow it
	unsigned int counter;
	//put('e');
	//put(which + '0');
	switch (which)
	{	//do any necessary interrupt masking first
		case SL_IRQ1:
			asm("cli");
			clearIRQ(1);
			break;
		case SL_MESSAGE: case SL_DISPLAY:
			asm("cli");
			break;
		default:
			break;
	}
	while ((test_and_set (1, &(spinlock_states[which].imp_enabled))) )
	{	//&(spinlock_states[which].imp_enabled)
		//spinlock_states[which].delays++;
		put('?');
	}
	//put('E');	//
	//put(which + '0');
	spinlock_states[which].exp_enabled = 1;
	//manually set the explicit enable flag after entering the
	//protected side of the spinlock
	for (counter = (which + 1); counter > 0; counter--)
	{	//set all lesser important spinlocks to implicitly locked
		spinlock_states[which - 1].imp_enabled = 1;
	}
}

void leave_spinlock(unsigned int which)
{	//can't leave a spinlock level if we are not there already
	unsigned int counter;
	//put('x');
	//put(which + '0');
	if (spinlock_states[which].exp_enabled != 1)
	{
		put('?');
		put('?');
		put('?');
		put('?');
		put('?');
		put('?');
		put('?');
		put('?');
		put('\n');
		while (1){};
	}
	else
	{	//clear the current spinlock, check for the next highest explicitly set spinlock
		//put('X');
		//put(which + '0');
		spinlock_states[which].exp_enabled = 0;
		spinlock_states[which].imp_enabled = 0;
		counter = which;
		while (counter != 0)
		{	//process until you hit the most interruptable spinlock
			//don't count the spinlock that was already explicitly set
			if (spinlock_states[counter].exp_enabled == 1)
			{	//the search is over
				break;
			}
			else
			{
				spinlock_states[counter].exp_enabled = 0;
				spinlock_states[counter].imp_enabled = 0;
				counter--;
			}
		}
	}
	switch (which)
	{
		case SL_IRQ1:
			enableIRQ(0);
			asm("sti");
			break;
		case SL_MESSAGE: case SL_DISPLAY:
			asm("sti");
			break;
		default:
			break;
	}
}

void initialize_spinlock()
{
	unsigned int counter;
	for (counter = 0; counter < NUMBER_TYPES; counter++)
	{
		spinlock_states[counter].exp_enabled = 0;
		spinlock_states[counter].imp_enabled = 0;
		spinlock_states[counter].delays = 0;
	}
}
