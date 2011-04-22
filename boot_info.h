     /* multiboot.h - the header for Multiboot */
     /* Copyright (C) 1999, 2001  Free Software Foundation, Inc.
     
        This program is free software; you can redistribute it and/or modify
        it under the terms of the GNU General Public License as published by
        the Free Software Foundation; either version 2 of the License, or
        (at your option) any later version.
     
        This program is distributed in the hope that it will be useful,
        but WITHOUT ANY WARRANTY; without even the implied warranty of
        MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
        GNU General Public License for more details.
     
        You should have received a copy of the GNU General Public License
        along with this program; if not, write to the Free Software
        Foundation, Inc., 675 Mass Ave, Cambridge, MA 02139, USA. */
     
     /* Macros. */
     
     /* The magic number for the Multiboot header. */
#ifndef _boot_info_
#define _boot_info_
     #define MULTIBOOT_HEADER_MAGIC          0x1BADB002
     
     /* The flags for the Multiboot header. */
     #ifdef __ELF__
     # define MULTIBOOT_HEADER_FLAGS         0x00000003
     #else
     # define MULTIBOOT_HEADER_FLAGS         0x00010003
     #endif
     
     /* The magic number passed by a Multiboot-compliant boot loader. */
     #define MULTIBOOT_BOOTLOADER_MAGIC      0x2BADB002
     
     /* The size of our stack (16KB). */
     #define STACK_SIZE                      0x4000
     
     /* C symbol format. HAVE_ASM_USCORE is defined by configure. */
     #ifdef HAVE_ASM_USCORE
     # define EXT_C(sym)                     _ ## sym
     #else
     # define EXT_C(sym)                     sym
     #endif
     
     #ifndef ASM
     /* Do not include here in boot.S. */
     
     /* Types. */
     
     /* The Multiboot header. */
     typedef struct multiboot_header
     {
       unsigned long magic;
       unsigned long flags;
       unsigned long checksum;
       unsigned long header_addr;
       unsigned long load_addr;
       unsigned long load_end_addr;
       unsigned long bss_end_addr;
       unsigned long entry_addr;
     } __attribute__((packed)) multiboot_header_t;
     
     /* The symbol table for a.out. */
     typedef struct aout_symbol_table
     {
       unsigned long tabsize;
       unsigned long strsize;
       unsigned long addr;
       unsigned long reserved;
     } __attribute__((packed)) aout_symbol_table_t;
     
     /* The section header table for ELF. */
     typedef struct elf_section_header_table
     {
       unsigned long num;
       unsigned long size;
       unsigned long addr;
       unsigned long shndx;
     } __attribute__((packed)) elf_section_header_table_t;
     
     /* The Multiboot information. */
     typedef struct multiboot_info
     {	//structure was modified according to the multiboot specification
				//notably, extra information was added to the end, and if the corresponding bits are not set in the flags
				//unused bits are set to 0 by default anyways
				//then nothing will change
       unsigned long flags;
			// when bit 0 of flags is set
       unsigned long mem_lower;	//number of kilobytes
       unsigned long mem_upper;
			//flags bit 1
       unsigned long boot_device;
			//bit 2
       unsigned long cmdline;
			//bit 3
       unsigned long mods_count;
       unsigned long mods_addr;
			//bit 4 or 5
       union
       {
         aout_symbol_table_t aout_sym;
         elf_section_header_table_t elf_sec;
       } u;
			//when bit 6 of flags is set, these two variables point to an array of memory_map_t's
       unsigned long mmap_length;
       unsigned long mmap_addr;
			//bit 7
			unsigned long drives_length;
			unsigned long drives_addr;
			//bit 8
			unsigned long config_table;
			//bit 9
			unsigned long boot_loader_name;
			//bit 10
			unsigned long apm_table;
			//bit 11
			unsigned long vbe_control_info;
			unsigned long vbe_mode_info;
			unsigned long vbe_mode;
			unsigned long vbe_interface_seg;
			unsigned long vbe_interface_off;
			unsigned long vbe_interface_len;
     } __attribute__((packed)) multiboot_info_t;
     
     /* The module structure. */
     typedef struct module
     {
       unsigned long mod_start;
       unsigned long mod_end;
       unsigned long string;
       unsigned long reserved;
     } __attribute__((packed)) module_t;
     
     /* The memory map. Be careful that the offset 0 is base_addr_low
        but no size. */
     typedef struct memory_map
     {
       unsigned long size;					//used to skip to the next block of memory addresses
       unsigned long base_addr_low;
       unsigned long base_addr_high;
       unsigned long length_low;
       unsigned long length_high;
       unsigned long type;					//1 indicates usable RAM
     } __attribute__((packed)) memory_map_t;
     
     #endif /* ! ASM */

#endif
