MEMORY
{
  ram (!rx) : ORIGIN = 0x20020000, LENGTH = 0x5c000
  ram2 (!rx) : ORIGIN = 0x2007c000, LENGTH = 0x4000
  flash_itcm (rx) : ORIGIN = 0x200000, LENGTH = 0x200000
  flash (rx) : ORIGIN = 0x008000000, LENGTH = 0x200000
}

SECTIONS
{
  .text :
  {
    KEEP(*(.multiboot));

    *(.text);
    *(.text.*);
    *(.got);
    *(.got.plt);
  } > flash
  .strings : {
    *(.dynstr);
  } > flash
  .data (NOLOAD) : { 
    *(.data);
    *(.data.*);
  } > ram
  .rodata : { *(.rodata) } > flash
  .rela : { *(.rela.dyn); } > flash
  .strtab : { *(.strtab); } > flash
  .bss (NOLOAD) : { 
    *(.bss);
    *(.bss.*);
  } > ram
  /DISCARD/ :
  {
    *(.dynamic);
    *(.dynsym);
    *(.gnu.hash);
    *(.comment);
    *(.hash);
    *(.debug*);
  }
}