MEMORY
{
  ram (!rx) : org = 0x20020000, l = 0x5c000
  flash (x) : org = 0x00200000, l = 0x200000
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