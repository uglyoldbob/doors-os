#ifdef __cplusplus
#define EXTERNC extern "C"
#else
#define EXTERNC
#endif 

//#ifndef _KEYBOARD_H_
//#define _KEYBOARD_H_
struct message
{
	unsigned int who;			//determines who the message is from (keyboard...)
	unsigned int fields;	//used to figure out how many of the following fields are valid
	unsigned int data1;		//data required for the message to make sense
	unsigned int data2;		//more data
};

#define KEYBOARD	1
#define SERIAL		2

EXTERNC void add_system_event(struct message *add_me);
//adds a message to the kernel message buffer

EXTERNC void check_system_event(unsigned int *ret_val);
//returns how many events are currently in the buffer

EXTERNC void get_system_event(struct message* move_here);
//copies a message to the given message structure

EXTERNC void init_messaging();

//#endif
