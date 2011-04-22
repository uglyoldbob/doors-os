echo off
ECHO This version of Doors requires a Pentium or greater processor
ECHO Delete old OS files
del D:\FAT12\*.bin
del D:\FAT12\*.o
ECHO Compile the bootsector
NASM -f bin D:\FAT12\bootload.asm -o D:\FAT12\botstrap.bin
NASM -f bin D:\FAT12\sample.asm -o D:\FAT12\second.bin
partcopy D:\Dos_boot.bin 0 200 -f0
partcopy D:\FAT12\botstrap.bin 0 3 -f0
partcopy D:\FAT12\botstrap.bin 3e 1c2 -f0 3e
ECHO We will extract our modified bootsector to the FAT12 directory
partcopy -f0 0 200 D:\FAT12\btstrp2.bin 

ECHO We will compile the kernel now

NASM -f aout D:\FAT12\kernel.asm -o D:\FAT12\kernel.o
gcc -c D:\FAT12\support.c -I./Inc -ffreestanding -nostdlib -fno-builtin -fno-exceptions -ffast-math -O3
gcc -c D:\FAT12\video.cpp -I./Inc -nostdlib -fno-builtin -fno-rtti -fno-exceptions -ffast-math -O3
rem -I./Inc -ffreestanding -nostdlib -fno-builtin -fno-exceptions -ffast-math -O3
gcc -c D:\FAT12\Ckernel.cpp -I./Inc -nostdlib -fno-builtin -fno-rtti -fno-exceptions -ffast-math -O3
ld -T link.ld -o doors.bin kernel.o Ckernel.o video.o support.o

pause

copy D:\FAT12\doors.bin D:\disk\floppy\doors.bin
copy D:\FAT12\second.bin D:\disk\floppy\second.bin

ECHO We will copy the contents of our bootable
ECHO floppy disk to the floppy disk, and to I:\doors.img
D:\bfi -t="144" -f=D:\Doors.img -b=D:\FAT12\btstrp2.bin D:\disk\floppy\
copy D:\disk\floppy\*.* A:\ /Y
ECHO Delete middle-layer OS files
del *.o
del *.bin
ECHO I'm done!

"C:\Program Files\Bochs-2.1.1\bochs.exe" -f "D:\bochs\fat12\info.txt" -q
