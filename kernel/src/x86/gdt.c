#include "video.h"
#include "tss.h"

unsigned short previous_tss = 0;
unsigned long previous = 0;
unsigned long task_timer = 50;
unsigned char enable_multi = 0;
struct multi_gdt *setup;

unsigned int timer = 0;

struct gdt_entry
{
	unsigned short 	field1;
	unsigned short 	field2;
	unsigned char 	field3;
	unsigned char 	field4;
	unsigned char 	field5;
	unsigned char 	field6;
} __attribute__((packed));


struct multi_gdt
{
	struct gdt_entry entries[5];
	unsigned short size;
	unsigned long address;
	struct TSS tss_info[2];
} __attribute__((packed));

struct TSS *get_current_tss()
{
	if (previous_tss == 1)
	{
		display("Current TSS: ");
		PrintNumber(&setup->tss_info[1]);
		display("\n");
		return &setup->tss_info[1];
	}
	else
	{
		display("Current TSS: ");
		PrintNumber(&setup->tss_info[0]);
		display("\n");
		return &setup->tss_info[0];
	}
}

unsigned int setup_multi_gdt()
{
	//setup structures
	setup = (struct multi_gdt*)0	;
//	multi_tss_begin = &(setup->tss_info[0]);
//	multi_tss2_begin = &(setup->tss_info[1]);
	//null segment
	setup->entries[0].field1 = 0;
	setup->entries[0].field2 = 0;
	setup->entries[0].field3 = 0;
	setup->entries[0].field4 = 0;
	setup->entries[0].field5 = 0;
	setup->entries[0].field6 = 0;
	//code segment
	setup->entries[1].field1 = 0xFFFF;
	setup->entries[1].field2 = 0;
	setup->entries[1].field3 = 0;
	setup->entries[1].field4 = 0x9A;
	setup->entries[1].field5 = 0xCF;
	setup->entries[1].field6 = 0;
	//data segment
	setup->entries[2].field1 = 0xFFFF;
	setup->entries[2].field2 = 0;
	setup->entries[2].field3 = 0;
	setup->entries[2].field4 = 0x92;
	setup->entries[2].field5 = 0xCF;
	setup->entries[2].field6 = 0;
	//tss1
	setup->entries[3].field1 = sizeof(struct TSS) + 1;
	setup->entries[3].field2 = (unsigned long)&setup->tss_info[0];
	setup->entries[3].field3 = 0;
	setup->entries[3].field4 = 0x89;
	setup->entries[3].field5 = 0;
	setup->entries[3].field6 = 0;
	//tss2
	setup->entries[4].field1 = sizeof(struct TSS) + 1;;
	setup->entries[4].field2 = (unsigned long)&setup->tss_info[1];
	setup->entries[4].field3 = 0;
	setup->entries[4].field4 = 0x89;
	setup->entries[4].field5 = 0;
	setup->entries[4].field6 = 0;

	setup->size = (sizeof(struct gdt_entry) * 5) - 1;
	setup->address = setup;

	setup_gdt(&setup->size);
	return 0;
}

void print_tss(struct TSS *me)
{
	display("Previous task: ");
	PrintNumber(me->previous_task);
	display("\tESP0: ");
	PrintNumber(me->ESP0);
	display("\tSS0: ");
	PrintNumber(me->SS0);	

	display("\tESP1: ");
	PrintNumber(me->ESP1);
	display("\tSS1: ");
	PrintNumber(me->SS1);	

	display("\tESP1: ");
	PrintNumber(me->ESP1);
	display("\tSS1: ");
	PrintNumber(me->SS1);	

	display("\nCR3: ");
	PrintNumber(me->cr3);

	display("\teip: ");
	PrintNumber(me->eip);
	display("\teflags: ");
	PrintNumber(me->eflags);
	display("\neax: ");
	PrintNumber(me->eax);
	display("\tebx: ");
	PrintNumber(me->ebx);
	display("\tecx: ");
	PrintNumber(me->ecx);
	display("\tedx: ");
	PrintNumber(me->edx);
	display("\nesp: ");
	PrintNumber(me->esp);
	display("\tebp: ");
	PrintNumber(me->ebp);
	display("\tesi: ");
	PrintNumber(me->esi);
	display("\tedi: ");
	PrintNumber(me->edi);
	display("\ncs: ");
	PrintNumber(me->cs);
	display("\tds: ");
	PrintNumber(me->ds);
	display("\tes: ");
	PrintNumber(me->es);
	display("\tfs: ");
	PrintNumber(me->fs);
	display("\tgs: ");
	PrintNumber(me->gs);
	display("\nldt_segment_selector: ");
	PrintNumber(me->ldt_segment_selector);
	display("\tdebug_trap: ");
	PrintNumber(me->debug_trap);
	display("\tio_map_base_address: ");
	PrintNumber(me->io_map_base_address);
	display("\n");
}

void irqM0_handler()
{	//handles the stuff that doesn't have to be done in assembly
	//called from an isr, which is already non-reentrant
		//so it doesnt have to be reentrant
	if (enable_multi == 0)
	{
		task_timer = 50;
		outportb(0x20,0x20);
		/*asm("pushw %ax               #save ax");
		asm("movb $0x20,%al");
		asm("outb %al, $0x20");
		asm("popw %ax                #restore ax");*/

		return;
	}
	if (task_timer != 0)
	{
		outportb(0x20,0x20);
		/*asm("pushw %ax               #save ax");
		asm("movb $0x20,%al");
		asm("outb %al, $0x20");
		asm("popw %ax                #restore ax");*/

		return;
	}
	task_timer = 50;
	if (previous_tss == 1)
	{
		previous_tss = 2;
		sys_tasks = next_state(previous, sys_tasks, &(setup->tss_info[1]), &(setup->tss_info[0]));
	}
	else
	{
		previous_tss = 1;
		sys_tasks = next_state(previous, sys_tasks, &(setup->tss_info[0]), &(setup->tss_info[1]));
	}
	previous = 1;
	//TODO: write code to print the TSS of the task being switched to

	outportb(0x20,0x20);
	/*asm("pushw %ax               #save ax");
	asm("movb $0x20,%al");
	asm("outb %al, $0x20");
	asm("popw %ax                #restore ax");*/
	if (previous_tss == 2)
	{
		//asm("ljmp $0x18, $00");
	}
	else
	{
		//asm("ljmp $0x20, $00");
	}
}

