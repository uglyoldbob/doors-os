#include "spinlock.h"
#include "message.h"
#include "memory.h"
#include "video.h"
//message.c
//these spinlock controlled functions will control the messaging system for the kernel

//add_system_event(unsigned int code)
//check_system_event()	- any code that is already there is not removed
//get_system_event()	- only returns if a system event is present, the code is removed

unsigned int *head_messages;
unsigned int length;	//bytes
//defines what addresses can possibly contain messages
struct message *messages;
unsigned int num_messages;
//defines where message are currently stored
 
void init_messaging()
{
	head_messages = (unsigned int*)kmalloc(0x1000);	
	//dynamically allocate 1 page to hold messages
	//it will probably never be filled
	length = 0x1000;
	messages = (struct message*)head_messages;
	num_messages = 0;
}

int add_system_event(struct message *add_me)
{
	enter_spinlock(SL_MESSAGE);
	if (num_messages == 0)
	{	//reset the buffer pointer
		messages = (struct message*)head_messages;
	}
	if ((num_messages * sizeof(unsigned int)) >= length)
	{
		display("Message buffer full\n");
		return -1;
	}
	messages[num_messages].who = add_me->who;
	messages[num_messages].fields = add_me->fields;
	messages[num_messages].data1 = add_me->data1;
	if (add_me->fields > 1)
		messages[num_messages].data2 = add_me->data2;
	num_messages++;	
	leave_spinlock(SL_MESSAGE);
	return 0;
}

void check_system_event(unsigned int *ret_val)
{
	enter_spinlock(SL_MESSAGE);
	*ret_val = num_messages;
	leave_spinlock(SL_MESSAGE);
}

void get_system_event(struct message* move_here)
{	//moves a system message to *move_here
	enter_spinlock(SL_MESSAGE);
	move_here->who = messages[0].who;
	move_here->fields = messages[0].fields;
	move_here->data1 = messages[0].data1;
	if (messages[0].fields > 1)
		move_here->data2 = messages[0].data2;
	messages++;
	num_messages--;
	leave_spinlock(SL_MESSAGE);
}
