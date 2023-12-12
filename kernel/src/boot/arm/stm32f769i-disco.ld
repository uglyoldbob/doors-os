MEMORY
{
  ram (!rx) : ORIGIN = 0x20020000, LENGTH = 0x5c000
  ram2 (!rx) : ORIGIN = 0x2007c000, LENGTH = 0x4000
  flash (rx) : ORIGIN = 0x00200000, LENGTH = 0x200000
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
  .dynamic :
  {
    
  } > ram
  .strings : {
    *(.dynstr);
  } > ram
  .data : { 
    *(.data);
    *(.data.*);
  } > ram
  .rodata : { *(.rodata) } > ram
  .rela : { *(.rela.dyn); } > ram
  .strtab : { *(.strtab); } > ram
  .bss : { 
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