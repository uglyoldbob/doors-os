/* Link.ld */

OUTPUT_FORMAT("elf32-i386")
ENTRY(start)

SECTIONS
{
.text 0x0100000 :
{
code = .; _code = .; __code = .;
*(.text)
. = ALIGN(4096);
}

.data :
{
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
