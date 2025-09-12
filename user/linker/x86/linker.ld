MEMORY
{
  ram (!rx) : org = 0x8000000000, l = 3M
}

SECTIONS
{
  . = 0x00500000;
  .text :
  {
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
  }
}