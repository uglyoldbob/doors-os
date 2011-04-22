#Doors Os Makefile
#loads from a floppy FLOPPY using GRUB
#uses FAT12 for a  filesystem

#the assembler to use
ASM = nasm

#NASM -f aout D:\FAT12\v.04\kernel.asm -o D:\FAT12\v.04\kernel.o
#gcc -c D:\FAT12\v.04\support.c -I./Inc -ffreestanding -nostdlib -fno-builtin -fno-exceptions -ffast-math -O3 
#gcc -c D:\FAT12\v.04\video.cpp -I./Inc -nostdlib -fno-builtin -fno-rtti -fno-exceptions -ffast-math -O3 
#gcc -c D:\FAT12\v.04\Ckernel.cpp -I./Inc -nostdlib -fno-builtin -fno-rtti -fno-exceptions -ffast-math -O3 
#ld -T link.ld -o doors.bin kernel.o Ckernel.o video.o support.o

# use "gcc" to compile source files.
CC = i386-elf-doors-gcc

# the linker is ld
LD = i386-elf-doors-ld

# Compiler flags go here.
KRNL_CFLAGS_DBG_C = -O0 -nostdlib -fno-builtin -fno-rtti -fno-exceptions -gdwarf-2
KRNL_CFLAGS_C = -O0 -nostdlib -fno-builtin -fno-rtti -fno-exceptions
KRNL_CFLAGS_DBG = -O0 -nostdlib -fno-builtin -fno-exceptions -gdwarf-2
KRNL_CFLAGS = -O0 -nostdlib -fno-builtin -fno-exceptions
	#C and c++ code, both types are compiled by the same compiler

#arguments to send to the assembler kernel program
KRNL_ASFLAGS_DBG = -O0 -f elf
KRNL_ASFLAGS = -O0 -f elf
#flags for the assembler for applications
APP_ASFLAGS_DBG = -O0 -f elf
APP_ASFLAGS = -O0 -f elf
#compiler flags for applications
APP_CFLAGS = -O0 -gdwarf-2
APP_CFLAGS = -O0 -gdwarf-2

# Linker flags go here. -s strips debug info
KRNL_LDFLAGS = -T link.ld
#linker flags for applications
APP_LDFLAGS = -T app-link.ld


# use this command to erase files.
RM = /bin/rm -f

# list of generated object files. they are seperated by what kind of file they are created from
OBJS = i386-stub.o interrupt_table.o boot_info.o dma.o PIC.o keyboard.o spinlock.o message.o support.o 

OBJS_C = filesystem.o video.o disk.o floppy.o memory.o fat.o main.o tss.o serial.o gdb-support.o

OBJS_ASM = entrance.o

#these are all of the c source files
SRCS = i386-stub.c interrupt_table.c boot_info.c dma.c PIC.c keyboard.c spinlock.c message.c support.c
#here are the c++ source files
SRCS_C = filesystem.cpp video.cpp disk.cpp floppy.cpp memory.cpp fat.cpp main.cpp tss.cpp serial.cpp gdb-support.cpp
#here are assembly files
SRCS_ASM = entrance.asm

# KERNELram executable file name.
KERNEL = kernel.bin

#name of the FLOPPY image file to be modified
FLOPPY = skeleton.img

#name of the CD image file to be created
CD = cdBoot.iso

#name of the test program to compile
TEST = test.bin

#the mount point to use for the virtual floppy drive
MNTSPOT = /mnt/floppy

# top-level rule, to compile everything.
all: $(FLOPPY) $(CD) $(TEST)

#just compile what is needed for the kernel
kernel: $(KERNEL)

#make sure the floppy image is fresh, then call bochs
floppy: $(FLOPPY)
	bochs -f floppy.txt

#same thing as floppy
cd: $(CD)
	bochs -f cd.txt

tftp: $(KERNEL)
	cp kernel.bin /tftpboot

test: $(TEST)
	echo "Test has been compiled"


#rule to modify the boot image
$(FLOPPY): $(KERNEL)
	sudo mount $(FLOPPY) -t msdos $(MNTSPOT) -o loop
	sudo cp $(KERNEL) $(MNTSPOT)/$(KERNEL)
	sudo umount -d $(MNTSPOT)

$(CD): $(KERNEL)
	#mkisofs -R -b boot/grub/stage2_eltorito -no-emul-boot \
        # -boot-load-size 4 -boot-info-table -o grub.iso iso
	genisoimage -R -b boot/grub/stage2_eltorito -no-emul-boot \
         -boot-load-size 4 -boot-info-table -o grub.iso iso


# rule to link the KERNEL
$(KERNEL): $(OBJS) $(OBJS_C) $(OBJS_ASM)
	$(LD) $(KRNL_LDFLAGS) $(OBJS_ASM) $(OBJS) $(OBJS_C) -o $(KERNEL)
	cp $(KERNEL) iso/$(KERNEL)

$(TEST): test.o
	$(LD) $(APP_LDFLAGS) test.o ../cross/i386-elf-doors/lib/libc.a -o $(TEST)

#test program to ensure that the c library can be compiled against
#just the fact that this compiles does not ensure proper operation of the C library
test.o: test.c
	$(CC) $(APP_CFLAGS) -c test.c -o test.o

#c code (except for c entries located above this
$(OBJS): $(SRCS)
	$(CC) $(KRNL_CFLAGS_DBG) -c $*.c

#c++ code except for all C++ entries found above this
$(OBJS_C): $(SRCS_C)
	$(CC) $(KRNL_CFLAGS_DBG_C) -c $*.cpp -o $*.o

#assembly code
$(OBJS_ASM): $(SRCS_ASM)
	$(ASM) $(KRNL_ASFLAGS) $*.asm -o $*.o


# rule for cleaning re-compilable (non-source) files.
clean:
	$(RM) $(KERNEL) $(OBJS) $(OBJS_C) $(CD) $(TEST) test.o
