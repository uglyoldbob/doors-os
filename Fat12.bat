echo off
ECHO Compile the bootsector
NASM -o D:\FAT12\v.03\botstrap.bin D:\FAT12\v.03\newstuf.asm
partcopy D:\Dos_boot.bin 0 200 -f0
partcopy D:\FAT12\v.03\botstrap.bin 0 3 -f0
partcopy D:\FAT12\v.03\botstrap.bin 3e 1c2 -f0 3e
ECHO We will extract our modified bootsector to the FAT12 directory
partcopy -f0 0 200 D:\FAT12\v.03\btstrp2.bin 

ECHO We will compile the kernel now
NASM -o D:\disk\floppy\doors.bin D:\FAT12\v.03\kernel.asm -s >log.txt

ECHO We will copy the contents of our bootable
ECHO floppy disk to the floppy disk, and to I:\doors.img
D:\bfi -t="144" -f=I:\Doors.img -b=D:\FAT12\v.03\btstrp2.bin D:\disk\floppy\
copy D:\disk\floppy\*.* A:\ /Y
del D:\FAT12\v.03\*.bin
ECHO I'm done!

"C:\Program Files\Bochs-2.1.1\bochsdbg.exe" -f "D:\bochs\fat12\info.txt" -q