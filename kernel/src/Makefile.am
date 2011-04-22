AUTOMAKE_OPTIONS = subdir-objects
AM_CFLAGS = -nostdlib -fno-builtin -fno-exceptions -gdwarf-2
AM_CXXFLAGS = -nostdlib -fno-builtin -fno-rtti -fno-exceptions -gdwarf-2
AM_CCASFLAGS = -gdwarf-2

AM_LDFLAGS = -T link.ld

CLEANFILES = $(bin_SCRIPTS)

bin_PROGRAMS = kernel
kernel_SOURCES = 	\
	boot_info.c 	\
	disk.cpp 		\
	elf.cpp 		\
	fat.cpp 		\
	file.cpp 		\
	filesystem.cpp 	\
	message.c 		\
	module.cpp 		\
	string.cpp 		\
	support.c
EXTRA_kernel_SOURCES =		\
	x86/entrance.s			\
	x86/gdt.c				\
	x86/dma.c				\
	x86/floppy.cpp			\
	x86/gdb-support.cpp		\
	x86/gdb-stub.c			\
	x86/interrupt_table.c	\
	x86/keyboard.c			\
	x86/main.cpp			\
	x86/memory.cpp			\
	x86/PIC.c				\
	x86/serial.cpp 			\
	x86/spinlock.c			\
	x86/tss.cpp				\
	x86/video.cpp			\
	x86/vmm.cpp				\
	x86/debug.c

kernel_LDADD = $(KERNEL_SYSTEM)
kernel_DEPENDENCIES = $(KERNEL_SYSTEM)

