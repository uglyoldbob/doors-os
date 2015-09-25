AUTOMAKE_OPTIONS = subdir-objects

AM_CPPFLAGS = -I${top_srcdir}/src

kernel_CFLAGS = \
	-std=c99 -ffreestanding \
	-O0 -nostdlib \
	-fno-builtin -fno-exceptions \
	-gdwarf-2 -Wall -Wextra
kernel_CXXFLAGS = \
	-O0 \
	-ffreestanding -fno-builtin -fno-rtti -fno-exceptions \
	-gdwarf-2  -Wall -Wextra
kernel_CCASFLAGS = -g -O0 -gdwarf-2
kernel_LDFLAGS = -T $(top_srcdir)/src/link.ld -nostdlib -ffreestanding -lgcc 

CLEANFILES = $(bin_SCRIPTS)

bin_PROGRAMS = kernel
if X86
kernel_SOURCES = \
	src/x86/entrance.s \
	src/x86/main.cpp \
	src/x86/debug.c \
	src/x86/dma.c \
	src/x86/floppy.cpp \
	src/x86/gdt.c \
	src/x86/interrupt_table.c \
	src/x86/keyboard.c \
	src/x86/memory.cpp \
	src/x86/new.cpp \
	src/x86/PIC.c \
	src/x86/serial.cpp \
	src/x86/spinlock.c \
	src/x86/tss.cpp \
	src/x86/video.cpp \
	src/x86/vmm.cpp
else
kernel_SOURCES =
	cannot_exist.c
endif

kernel_SOURCES += \
	src/boot_info.c \
	src/disk.cpp \
	src/elf.cpp \
	src/fat.cpp \
	src/file.cpp \
	src/filesystem.cpp \
	src/message.c \
	src/module.cpp \
	src/support.c \
	src/terminal.cpp \
	src/string/string.c \
	src/stdlib/stdlib.c

kernel_LDADD = $(KERNEL_SYSTEM)
kernel_DEPENDENCIES = $(KERNEL_SYSTEM)

itab.c itab.h: x86optable.xml opgen.py
	python ./opgen.py
	
qemu_floppy: floppy.img
	qemu-system-i386 -cpu qemu32 -s -fda floppy.img -boot a -m 4

floppy.img: kernel serial.so $(top_srcdir)/Grub.img 
	cp $(top_srcdir)/Grub.img floppy.img
	mount floppy.img -t msdos /media/floppy -o loop
	cp kernel /media/floppy/kernel.bin
	cp $(top_srcdir)/serial.so /media/floppy/serial.so
	until umount -d /media/floppy; do sleep 0.1; done
	

qemu_cd: doors.iso
	qemu-system-i386 -cpu qemu32 -s -cdrom doors.iso -boot d -m 4

doors.iso: kernel boot/grub/stage2_eltorito
	mkdir -p iso
	cp $(top_srcdir)/README iso/README
	cp $(top_srcdir)/serial.so iso/serial.so
	genisoimage -R -b boot/grub/stage2_eltorito -no-emul-boot \
			-boot-load-size 4 -boot-info-table -o doors.iso iso
	

