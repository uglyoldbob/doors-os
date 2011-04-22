#define NUM_INTS 256

//;20 - 31 are reserved, 32-39 used for master IRQ 0 - 7, 40 - 47 slave IRQ 0 - 7, 
//	;48-255 are usable for anything

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
}__attribute__((packed)) idt;

extern void isr0();
extern void isr1();
extern void isr2();
extern void isr3();
extern void isr4();
extern void isr5();
extern void isr6();
extern void isr7();
extern void isr8();
extern void isr9();
extern void isr10();
extern void isr11();
extern void isr12();
extern void isr13();
extern void isr14();
extern void isr15();
extern void isr16();
extern void isr17();
extern void isr18();
extern void isr19();

extern void irqM0();
extern void irqM1();
extern void irqM2();
extern void irqM3();
extern void irqM4();
extern void irqM5();
extern void irqM6();
extern void irqM7();
extern void irqM8();
extern void irqM9();
extern void irqM10();
extern void irqM11();
extern void irqM12();
extern void irqM13();
extern void irqM14();
extern void irqM15();
