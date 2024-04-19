MEMORY
{
  ram (!rx) : org = 0x100000, l = 3M
  debug (r) : org = 0x40000000, l = 1024M
}

SECTIONS
{
  . = 0x00100000;
  START_OF_KERNEL = .;
  .text :
  {
    KEEP(*(.multiboot));

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
  .end_kernel : {
    END_OF_KERNEL = .;
  } > ram
  .debug (NOLOAD) : {
    KEEP(*(.debug));
  } > debug
  /DISCARD/ :
  {
    *(.dynamic);
    *(.dynsym);
    *(.gnu.hash);
    *(.comment);
    *(.hash);
  }
}