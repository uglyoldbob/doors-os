AUTOMAKE_OPTIONS = subdir-objects
AM_CFLAGS = -std=c99 -O0 -nostdlib -fno-builtin -fno-exceptions -gdwarf-2
AM_CXXFLAGS = -O0 -nostdlib -fno-builtin -fno-rtti -fno-exceptions -gdwarf-2
AM_CCASFLAGS = -g -O0 -gdwarf-2

AM_LDFLAGS = -T link.ld

CLEANFILES = $(bin_SCRIPTS)

bin_PROGRAMS = kernel
kernel_SOURCES = \
	boot_info.c\
	disk.cpp\
	elf.cpp\
	fat.cpp\
	file.cpp\
	filesystem.cpp\
	message.c\
	module.cpp\
	support.c\
	stdio/stdio.c\
	stdlib/stdlib.c\
	string/string.c\
	assert/assert.c

EXTRA_kernel_SOURCES =	\
	x86/entrance.s\
	x86/gdt.c\
	x86/dma.c\
	x86/floppy.cpp\
	x86/interrupt_table.c\
	x86/keyboard.c\
	x86/main.cpp\
	x86/memory.cpp\
	x86/PIC.c\
	x86/serial.cpp\
	x86/spinlock.c\
	x86/tss.cpp\
	x86/video.cpp\
	x86/vmm.cpp\
	x86/debug.c\
	x86/disasm/decode.c\
	x86/disasm/input.c\
	x86/disasm/itab.c\
	x86/disasm/opgen.py\
	x86/disasm/syn.c\
	x86/disasm/syn-att.c\
	x86/disasm/syn-intel.c\
	x86/disasm/udis86.c

kernel_LDADD = $(KERNEL_SYSTEM)
kernel_DEPENDENCIES = $(KERNEL_SYSTEM)

itab.c itab.h: x86optable.xml opgen.py
	python ./opgen.py
