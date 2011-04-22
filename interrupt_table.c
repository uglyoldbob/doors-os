#include "interrupt_table.h"

struct idt_desc *setupIdt()
{
	int counter;
	idt.description.address = (unsigned int)(&(idt.list[0]));
	idt.description.length = (unsigned int)(&(idt.description)) - (unsigned int)(&(idt.list[0])) + 1;
	for (counter = 0; counter < NUM_INTS; counter++)
	{
		idt.list[counter].segment = 0x08;
		idt.list[counter].blank = 0;
	}
	idt.list[0].low_address = ((unsigned)isr0 & 0xFFFF);
	idt.list[0].upper_address = ((unsigned)isr0 >> 16);
	idt.list[0].flags = 0x8E;
	idt.list[1].low_address = ((unsigned)isr1 & 0xFFFF);
	idt.list[1].upper_address = ((unsigned)isr1 >> 16);
	idt.list[1].flags = 0x8E;
	idt.list[2].low_address = ((unsigned)isr2 & 0xFFFF);
	idt.list[2].upper_address = ((unsigned)isr2 >> 16);
	idt.list[2].flags = 0x8E;
	idt.list[3].low_address = ((unsigned)isr3 & 0xFFFF);
	idt.list[3].upper_address = ((unsigned)isr3 >> 16);
	idt.list[3].flags = 0x8E;
	idt.list[4].low_address = ((unsigned)isr4 & 0xFFFF);
	idt.list[4].upper_address = ((unsigned)isr4 >> 16);
	idt.list[4].flags = 0x8E;
	idt.list[5].low_address = ((unsigned)isr5 & 0xFFFF);
	idt.list[5].upper_address = ((unsigned)isr5 >> 16);
	idt.list[5].flags = 0x8E;
	idt.list[6].low_address = ((unsigned)isr6 & 0xFFFF);
	idt.list[6].upper_address = ((unsigned)isr6 >> 16);
	idt.list[6].flags = 0x8E;
	idt.list[7].low_address = ((unsigned)isr7 & 0xFFFF);
	idt.list[7].upper_address = ((unsigned)isr7 >> 16);
	idt.list[7].flags = 0x8E;
	idt.list[8].low_address = ((unsigned)isr8 & 0xFFFF);
	idt.list[8].upper_address = ((unsigned)isr8 >> 16);
	idt.list[8].flags = 0x8E;
	idt.list[9].low_address = ((unsigned)isr9 & 0xFFFF);
	idt.list[9].upper_address = ((unsigned)isr9 >> 16);
	idt.list[9].flags = 0x8E;
	idt.list[10].low_address = ((unsigned)isr10 & 0xFFFF);
	idt.list[10].upper_address = ((unsigned)isr10 >> 16);
	idt.list[10].flags = 0x8E;
	idt.list[11].low_address = ((unsigned)isr11 & 0xFFFF);
	idt.list[11].upper_address = ((unsigned)isr11 >> 16);
	idt.list[11].flags = 0x8E;
	idt.list[12].low_address = ((unsigned)isr12 & 0xFFFF);
	idt.list[12].upper_address = ((unsigned)isr12 >> 16);
	idt.list[12].flags = 0x8E;
	idt.list[13].low_address = ((unsigned)isr13 & 0xFFFF);
	idt.list[13].upper_address = ((unsigned)isr13 >> 16);
	idt.list[13].flags = 0x8E;
	idt.list[14].low_address = ((unsigned)isr14 & 0xFFFF);
	idt.list[14].upper_address = ((unsigned)isr14 >> 16);
	idt.list[14].flags = 0x8E;
	idt.list[15].low_address = 0;
	idt.list[15].upper_address = 0;
	idt.list[15].flags = 0x0E;
	idt.list[16].low_address = ((unsigned)isr16 & 0xFFFF);
	idt.list[16].upper_address = ((unsigned)isr16 >> 16);
	idt.list[16].flags = 0x8E;
	idt.list[17].low_address = ((unsigned)isr17 & 0xFFFF);
	idt.list[17].upper_address = ((unsigned)isr17 >> 16);
	idt.list[17].flags = 0x8E;
	idt.list[18].low_address = ((unsigned)isr18 & 0xFFFF);
	idt.list[18].upper_address = ((unsigned)isr18 >> 16);
	idt.list[18].flags = 0x8E;
	idt.list[19].low_address = ((unsigned)isr19 & 0xFFFF);
	idt.list[19].upper_address = ((unsigned)isr19 >> 16);
	idt.list[19].flags = 0x8E;
	for (counter = 20; counter < 32; counter++)
	{	//reserved entries
		idt.list[counter].low_address = 0;
		idt.list[counter].upper_address = 0;
		idt.list[counter].flags = 0x0E;
	}
	idt.list[32].low_address = ((unsigned)irqM0 & 0xFFFF);
	idt.list[32].upper_address = ((unsigned)irqM0 >> 16);
	idt.list[32].flags = 0x8E;
	idt.list[33].low_address = ((unsigned)irqM1 & 0xFFFF);
	idt.list[33].upper_address = ((unsigned)irqM1 >> 16);
	idt.list[33].flags = 0x8E;
	idt.list[34].low_address = ((unsigned)irqM2 & 0xFFFF);
	idt.list[34].upper_address = ((unsigned)irqM2 >> 16);
	idt.list[34].flags = 0x8E;
	idt.list[35].low_address = ((unsigned)irqM3 & 0xFFFF);
	idt.list[35].upper_address = ((unsigned)irqM3 >> 16);
	idt.list[35].flags = 0x8E;
	idt.list[36].low_address = ((unsigned)irqM4 & 0xFFFF);
	idt.list[36].upper_address = ((unsigned)irqM4 >> 16);
	idt.list[36].flags = 0x8E;
	idt.list[37].low_address = ((unsigned)irqM5 & 0xFFFF);
	idt.list[37].upper_address = ((unsigned)irqM5 >> 16);
	idt.list[37].flags = 0x8E;
	idt.list[38].low_address = ((unsigned)irqM6 & 0xFFFF);
	idt.list[38].upper_address = ((unsigned)irqM6 >> 16);
	idt.list[38].flags = 0x8E;
	idt.list[39].low_address = ((unsigned)irqM7 & 0xFFFF);
	idt.list[39].upper_address = ((unsigned)irqM7 >> 16);
	idt.list[39].flags = 0x8E;
	for (counter = 40; counter < NUM_INTS; counter++)
	{	//unused entries, these get filled out so the comp wont crash if they get called
		idt.list[counter].low_address = 0;
		idt.list[counter].upper_address = 0;
		idt.list[counter].flags = 0x0E;
	}
	return &(idt.description);
}
