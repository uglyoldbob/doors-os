#define NUM_INTS 256

//;20 - 31 are reserved, 32-39 used for master IRQ 0 - 7, 40 - 47 slave IRQ 0 - 7, 
//	;48-255 are usable for anything

#ifdef __cplusplus
#define EXTERNC extern "C"
#else
#define EXTERNC
#endif 

#ifndef _IDT_H_
#define _IDT_H_

struct idt_entry
{
	unsigned short low_address;
	unsigned short segment;
	unsigned char blank;
	unsigned char flags;
	unsigned short upper_address;
}__attribute__((packed));

struct idt_desc
{
	unsigned short length;
	unsigned int address;
} __attribute__((packed));

struct idt_s
{
	struct idt_entry list[NUM_INTS];
	struct idt_desc description;
}__attribute__((packed));

EXTERNC void isr0();
EXTERNC void isr1();
EXTERNC void isr2();
EXTERNC void isr3();
EXTERNC void isr4();
EXTERNC void isr5();
EXTERNC void isr6();
EXTERNC void isr7();
EXTERNC void isr8();
EXTERNC void isr9();
EXTERNC void isr10();
EXTERNC void isr11();
EXTERNC void isr12();
EXTERNC void isr13();
EXTERNC void isr14();
EXTERNC void isr15();
EXTERNC void isr16();
EXTERNC void isr17();
EXTERNC void isr18();
EXTERNC void isr19();

EXTERNC void irqM0();
EXTERNC void irqM1();
EXTERNC void irqM2();
EXTERNC void irqM3();
EXTERNC void irqM4();
EXTERNC void irqM5();
EXTERNC void irqM6();
EXTERNC void irqM7();
EXTERNC void irqM8();
EXTERNC void irqM9();
EXTERNC void irqM10();
EXTERNC void irqM11();
EXTERNC void irqM12();
EXTERNC void irqM13();
EXTERNC void irqM14();
EXTERNC void irqM15();

EXTERNC void syscall1();

EXTERNC struct idt_desc *setupIdt();
EXTERNC void set_int_handler(void *address, unsigned int which_int);

#endif

