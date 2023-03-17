/* Link.ld */

OUTPUT_FORMAT("elf32-i386")
ENTRY(start)

SECTIONS
{
	.text 0x100000:
	{ 
		LONG(0x1BADB002)
		LONG(0x00000003)
		LONG(0xE4524FFB)
		*(.text)
	}
	.data :
	{ 
		*(.data) 
	}
	.bss : 
	{ 
		*(.bss) 
	}
}
