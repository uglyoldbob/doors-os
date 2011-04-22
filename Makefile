#Doors Os Makefile
#loads from a floppy FLOPPY using GRUB
#uses FAT12 for a  filesystem

#the assembler to use
ASM = nasm

#arguments to send to the assembler
ASFLAGS = -O1 -f elf

# use "gcc" to compile source files.
CC = /usr/cross/bin/i386-elf-gcc

# the linker is ld
LD = /usr/cross/bin/i386-elf-ld

# Compiler flags go here.
CFLAGS =  -ffast-math -O2 
#-fno-builtin -nostdlib -fno-exceptions
# Linker flags go here. 
LDFLAGS = -s -T link.ld

# use this command to erase files.
RM = /bin/rm -f

# list of generated object files.
OBJS = entrance.o main.o video.o  interrupt_table.o memory.o boot_info.o floppy.o dma.o PIC.o keyboard.o spinlock.o message.o disk.o fat.o

#these are all of the c source files
SRCS = main.c video.c  interrupt_table.c memory boot_info.c floppy.c dma.c PIC.c keyboard.c spinlock.c message.c disk.c fat.c

# KERNELram executable file name.
KERNEL = kernel.bin

#name of the FLOPPY image file to be modified
FLOPPY = skeleton.img

#name of the CD image file to be created
CD = cdBoot.iso

#the mount point to use for the virtual floppy drive
MNTSPOT = /mnt/floppy

# top-level rule, to compile everything.
all: $(FLOPPY) $(CD)

#just compile what is needed for the kernel
kernel: $(KERNEL)

#make sure the floppy image is fresh, then call bochs
floppy: $(FLOPPY)
	bochs -f floppy.txt

#same thing as floppy
cd: $(CD)
	bochs -f cd.txt

#rule to modify the boot image
$(FLOPPY): $(KERNEL)
	sudo mount $(FLOPPY) -t msdos $(MNTSPOT) -o loop
	sudo cp $(KERNEL) $(MNTSPOT)/$(KERNEL)
	sudo umount -d $(MNTSPOT)

$(CD): $(KERNEL)
	mkisofs -R -b boot/grub/stage2_eltorito -no-emul-boot \
         -boot-load-size 4 -boot-info-table -o grub.iso iso


# rule to link the KERNEL
$(KERNEL): $(OBJS)
	$(LD) $(LDFLAGS) $(OBJS) -o $(KERNEL)
	cp $(KERNEL) iso/$(KERNEL)

#rule for entrance.o (assembly file)
entrance.o: entrance.asm
	$(ASM) $(ASFLAGS) entrance.asm -o entrance.o

#rule for all c code
$(SRCS):
	$(CC) $(CFLAGS) -c $(SRCS) -o $(SRCS).o

# rule for cleaning re-compilable files.
clean:
	$(RM) $(KERNEL) $(OBJS) $(CD)
