#Doors Os Makefile
#loads from a floppy FLOPPY using GRUB
#uses FAT12 for a  filesystem

#the assembler to use
ASM = nasm

# use "gcc" to compile source files.
CC = i386-elf-doors-gcc

# the linker is ld
LD = i386-elf-doors-ld

# Compiler flags go here.
KRNL_CFLAGS =  -O1 -nostdlib
#-fno-builtin -nostdlib -fno-exceptions

APP_CFLAGS = -O1

# Linker flags go here. 
KRNL_LDFLAGS = -s -T link.ld

APP_LDFLAGS = -s -T app-link.ld

#arguments to send to the assembler
KRNL_ASFLAGS = -O1 -f elf

APP_ASFLAGS = -O1 -f elf

# use this command to erase files.
RM = /bin/rm -f

# list of generated object files.
OBJS = entrance.o main.o video.o  interrupt_table.o memory.o boot_info.o floppy.o dma.o PIC.o keyboard.o spinlock.o message.o disk.o fat.o tss.o

#these are all of the c source files
SRCS = main.c video.c  interrupt_table.c memory boot_info.c floppy.c dma.c PIC.c keyboard.c spinlock.c message.c disk.c fat.c tss.c

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
$(KERNEL): $(OBJS)
	$(LD) $(KRNL_LDFLAGS) $(OBJS) -o $(KERNEL)
	cp $(KERNEL) iso/$(KERNEL)

$(TEST): test.o
	$(LD) $(APP_LDFLAGS) test.o ../cross/i386-elf-doors/lib/libc.a -o $(TEST)

#rule for entrance.o (assembly file)
entrance.o: entrance.asm
	$(ASM) $(KRNL_ASFLAGS) entrance.asm -o entrance.o

test.o: test.c
	$(CC) $(APP_CFLAGS) -c test.c -o test.o

#rule for all c code
$(SRCS):
	$(CC) $(KRNL_CFLAGS) -c $(SRCS) -o $(SRCS).o

# rule for cleaning re-compilable files.
clean:
	$(RM) $(KERNEL) $(OBJS) $(CD) $(TEST) test.o
