/* Link.ld */

OUTPUT_FORMAT("elf32-i386")
ENTRY(start)

SECTIONS
{
.text 0x0100000 :
{
LONG(0x1BADB002)
LONG(0x00000003)
LONG(0xE4524FFB)
code = .; _code = .; __code = .;
*(.text)
*(.rodata)
. = ALIGN(4096);
}

.data :
{
__CTOR_LIST__ = .; LONG((__CTOR_END__ - __CTOR_LIST__) / 4 - 2) *(.ctors) LONG(0) __CTOR_END__ = .; 
__DTOR_LIST__ = .; LONG((__DTOR_END__ - __DTOR_LIST__) / 4 - 2) *(.dtors) LONG(0) __DTOR_END__ = .; 

data = .; _data = .; __data = .;
*(.data)
. = ALIGN(4096);
}

.rodata :
{
*(.rodata)
}

.bss :
{
bss = .; _bss = .; __bss = .;
*(.bss)
. = ALIGN(4096);
}


end = .; _end = .; __end = .;
kernel_end = .;
}
