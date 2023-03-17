cp Grub.img skeleton.img
sudo mount skeleton.img -t msdos /mnt/floppy -o loop
cp src/kernel kernel.bin
strip kernel.bin
sudo cp kernel.bin /mnt/floppy/kernel.bin
cp src/kernel kernel.bin
sudo cp ./serial.so /mnt/floppy/serial.so
sudo umount -d /mnt/floppy
bochs-gdb -f floppy.txt -q
qemu -cpu qemu32 -s -fda skeleton.img -boot a -m 4
