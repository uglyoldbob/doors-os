mkdir iso
cp src/kernel kernel.bin
strip kernel.bin
cp kernel.bin iso/kernel.bin
cp src/kernel kernel.bin
cp README iso/README
cp serial.so iso/serial.so
genisoimage -R -b boot/grub/stage2_eltorito -no-emul-boot \
			-boot-load-size 4 -boot-info-table -o doors.iso iso
bochs -f cd.txt
qemu -cpu qemu32 -s -cdrom doors.iso -boot d -m 4
