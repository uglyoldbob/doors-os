MEMORY
{
  ram (!rx) : org = 0x100000, l = 3M
}

SECTIONS
{
  . = 0x00100000;
  .text :
  {
    #multiboot 1
    LONG(0x1BADB002)
    LONG(0x00000003)
    LONG(0xE4524FFB)

    #multiboot2
    #LONG(0xe85250d6)
    #LONG(0)
    #LONG(0x10)
    #LONG(0xffffffff - 0x10 - 0xe85250d6 + 1)

    *(.text);
    *(.text.*);
    *(.got);
    *(.got.plt);
  } > ram
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