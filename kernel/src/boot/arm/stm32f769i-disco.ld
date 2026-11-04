MEMORY
{
  ram (!rx) : ORIGIN = 0x20020000, LENGTH = 0x5c000
  ram2 (!rx) : ORIGIN = 0x2007c000, LENGTH = 0x4000
  flash_itcm (rx) : ORIGIN = 0x200000, LENGTH = 0x200000
  flash (rx) : ORIGIN = 0x008000000, LENGTH = 0x200000
}

SECTIONS
{
  .vector_table :
  {
    // The initial stack pointer, the top of the dedicated ram2 stack region. This must stay
    // 8 byte aligned, the cortex-m core faults on the push instructions that an unaligned
    // stack pointer would produce.
    LONG(0x20080000);
    LONG(_start);
    . = ALIGN(4);
    KEEP(*(.isr_vector)) /* Startup code */
    . = ALIGN(4);
  } > flash

  .text :
  {
    KEEP(*(.multiboot));

    *(.text);
    *(.text.*);
    *(.got);
    *(.got.plt);
  } > flash
  .rodata : { *(.rodata) } > flash
  .rela : { *(.rela.dyn); } > flash
  .strtab : { *(.strtab); } > flash
  RAMLOAD = .;
  .strings : {
    *(.dynstr);
  } > flash
  .data : AT(RAMLOAD) { 
    *(.data);
    *(.data.*);
  } > ram
  .bss : { 
    START_OF_BSS = .;
    *(.bss);
    *(.bss.*);
    END_OF_BSS = .;
  } > ram
  .debug (NOLOAD) : {
    KEEP(*(*.debug));
  }
  /DISCARD/ :
  {
    *(.dynamic);
    *(.dynsym);
    *(.gnu.hash);
    *(.comment);
    *(.hash);
    *(.ARM.exidx);
  }
}