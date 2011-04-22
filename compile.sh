#insert the location of this file here
location="/home/thomas/DoorsOs/FAT12GRUB"
#where the floppy disk gets mounted to (virtual)
floppyDisk="/dev/loop/0"
#the other virtual virtual floppy disk
floppyDisk2="/mnt/floppy"

nasm -O1 -f aout "$location"/entrance.asm -o "$location"/entrance.o

gcc -c "$location"/main.c -fno-builtin -nostdlib -fno-exceptions -ffast-math -O3 -o "$location"/main.o
gcc -c "$location"/video.c -fno-builtin -nostdlib -fno-exceptions -ffast-math -O3 -o "$location"/video.o
gcc -c "$location"/interrupt_table.c -fno-builtin -nostdlib -fno-exceptions -ffast-math -O3 -o "$location"/interrupt_table.o
gcc -c "$location"/memory.c -fno-builtin -nostdlib -fno-exceptions -ffast-math -O3 -o "$location"/memory.o
gcc -c "$location"/boot_info.c -fno-builtin -nostdlib -fno-exceptions -ffast-math -O3 -o "$location"/boot_info.o
gcc -c "$location"/floppy.c -fno-builtin -nostdlib -fno-exceptions -ffast-math -O3 -o "$location"/floppy.o
gcc -c "$location"/dma.c -fno-builtin -nostdlib -fno-exceptions -ffast-math -O3 -o "$location"/dma.o
gcc -c "$location"/PIC.c -fno-builtin -nostdlib -fno-exceptions -ffast-math -O3 -o "$location"/PIC.o
gcc -c "$location"/keyboard.c -fno-builtin -nostdlib -fno-exceptions -ffast-math -O3 -o "$location"/keyboard.o
gcc -c "$location"/spinlock.c -fno-builtin -nostdlib -fno-exceptions -ffast-math -O3 -o "$location"/spinlock.o
gcc -c "$location"/message.c -fno-builtin -nostdlib -fno-exceptions -ffast-math -O3 -o "$location"/message.o


ld -T "$location"/link.ld -o "$location"/kernel.bin "$location"/entrance.o "$location"/main.o "$location"/video.o  "$location"/interrupt_table.o "$location"/memory.o "$location"/boot_info.o "$location"/floppy.o "$location"/dma.o "$location"/PIC.o "$location"/keyboard.o "$location"/spinlock.o "$location"/message.o
#sudo ld -T "$location"/OLD_link.ld -o "$location2"/doors.bin "$location2"/kernel.o "$location2"/Ckernel.o
#make sure image is not already mounted
#set +e
#sudo umount "$floppyDisk"
#sudo losetup -d "$floppyDisk"
#set -e
sudo losetup "$floppyDisk" "$location"/skeleton.img
sudo mount -t msdos "$floppyDisk" "$floppyDisk2"
sudo cp "$location"/kernel.bin "$floppyDisk2"/kernel.bin
sudo umount "$floppyDisk"
sudo losetup -d "$floppyDisk"
sleep 1
bochs -f "$location"/makeBoot.txt
