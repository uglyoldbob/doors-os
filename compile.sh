#insert where FAT12 directory is here
location="/home/thomas/DoorsOs/FAT12GRUB/v0.13"
#insert the location of this file here
location2="/home/thomas/DoorsOs/FAT12GRUB/v0.13"
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

ld -T "$location"/link.ld -o "$location"/kernel.bin "$location"/entrance.o "$location"/main.o "$location"/video.o  "$location"/interrupt_table.o "$location"/memory.o "$location"/boot_info.o
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
#echo "Compiling the kernel..."
#sudo nasm -O1 -f aout "$location"/kernel.asm -o "$location2"/kernel.o
#-ffreestanding
#-ffast-math
#sudo gcc -c "$location"/support.c -I./Inc -fno-builtin -nostdlib -fno-exceptions -ffast-math -O3 -o "$location2"/support.o
#sudo gcc -c "$location"/video.cpp -I./Inc -fno-builtin -nostdlib -fno-exceptions -ffast-math -O3 -o "$location2"/video.o
#sudo gcc -c "$location"/Ckernel.c -I./Inc -fno-builtin -nostdlib -fno-exceptions -O3 -o "$location2"/Ckernel.o
#kernel.o comes first because that is what needs to be at the top of the file, where execution will begin
#-T "$location"/link.ld
#$location2"/video.o "$location2"/support.o
#sudo ld -T "$location"/OLD_link.ld -o "$location2"/doors.bin "$location2"/kernel.o "$location2"/Ckernel.o
#echo "Creating floppy disk image..."
#make sure image is not already mounted
#set +e
#sudo umount "$floppyDisk"
#sudo losetup -d "$floppyDisk"
#set -e
#create a blank image
#sudo dd if=/dev/zero of="$location"/../Doors.img bs=512 count=2880
#mount that blank image and format it
#sudo losetup "$floppyDisk" "$location"/../Doors.img
#sudo mkdosfs "$floppyDisk"
#install bootsector onto floppy disk
#sudo dd if="$location2"/botstrap.bin of="$floppyDisk" bs=512
#now mount it
#sudo mount -t msdos "$floppyDisk" /mnt/floppy
#now copy all required files to the floppy disk
#sudo cp -f "$location2"/doors.bin /mnt/floppy/doors.bin
#sudo cp -f "$location2"/second.bin /mnt/floppy/second.bin
#unmount it so the image file can be used
#sudo umount "$floppyDisk"
#sudo losetup -d "$floppyDisk"
#sudo rm -f *.o
#echo "Starting emulator..."
#sudo bochs -f "$location"/../info2.txt -q
