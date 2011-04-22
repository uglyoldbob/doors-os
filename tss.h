#include "memory.h"

struct task *sys_tasks;

extern unsigned long *multi_tss_begin;	//located in entrance.asm
extern unsigned char enable_multi;
extern struct TSS *get_current_tss();
unsigned long get_sys_tasks();


struct TSS
{
	unsigned short previous_task;
	unsigned short reserved1;
	unsigned long ESP0;
	unsigned short SS0;
	unsigned short reserved2;
	unsigned long ESP1;
	unsigned short SS1;
	unsigned short reserved3;
	unsigned long ESP2;
	unsigned short SS2;
	unsigned short reserved4;
	unsigned long cr3;
	unsigned long eip;
	unsigned long eflags;
	unsigned long eax;
	unsigned long ecx;
	unsigned long edx;
	unsigned long ebx;
	unsigned long esp;
	unsigned long ebp;
	unsigned long esi;
	unsigned long edi;
	unsigned short es;
	unsigned short reserved5;
	unsigned short cs;
	unsigned short reserved6;
	unsigned short ss;
	unsigned short reserved7;
	unsigned short ds;
	unsigned short reserved8;
	unsigned short fs;
	unsigned short reserved9;
	unsigned short gs;	
	unsigned short reserved10;
	unsigned short ldt_segment_selector;
	unsigned short debug_trap;	//low bit is the flag, all else is reserved
	unsigned short io_map_base_address;
} __attribute__((packed));
//for now neither the IO or interrupt redirect bitmaps will be used
	//the IO bitmap allows a lesser privelaged tasks access to certain ports
	//the interrupt redirect is for V8086 mode. Interrupts will go to the handlers in 8086 mode or to the pmode handlers.
	//when they are used, unsigned char *io_map, char *int_redirect_map will be the format
		//appropriate code will have to be added

struct task
{	//circularly linked list
	struct task *previous;
	struct TSS *me;	//state of the task (if inactive)
	struct task *next;
};

int init_first_task(struct task *list);
void secondary_task();

//task management will center on the timer function (which will fire once per millisecond)
//maybe use the PIC for this instead

//to switch tasks,
//copy data from the current task to its place in the circularly linked list so it can be suspended (clear the busy flag)
//copy data from the circularly linked list to the TSS, set the busy bit
//set the NT flag in the EFLAGS register
//iret

//modify to use two TSS's (mix of hardware and software task switching)

