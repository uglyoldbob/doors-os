all: qemu32

qemu32: build/floppy.img
	qemu-system-i386 -fda build/floppy.img -m 4 -d cpu_reset
qemu64: build/floppy.img
	qemu-system-x86_64 -fda build/floppy.img -m 4 -d cpu_reset

qemucd32: build/cd.img
	qemu-system-i386 -cdrom build/cd.img -m 4 -d cpu_reset

qemucd: build/cd.img
	qemu-system-x86_64 -cdrom build/cd.img -m 4 -d cpu_reset

bochs: build/cd.img
	bochs -f bochsrc.txt -q

gdb: build/cd.img
	gdb -x script.gdb

kernel:
	mkdir -p ./build
	cd kernel; cargo build --release
	cp -u target/i386-unknown-none/release/kernel ./build/kernel

.PHONY: kernel

build/cd.img: kernel
	mkdir -p build/iso/boot/grub
	cp grub2.lst ./build/iso/boot/grub/grub.cfg
	cp ./build/kernel ./build/iso/boot/kernel
	grub-mkrescue -o ./build/cd.img build/iso
	rm -rf ./build/iso

build/grub.img:
	wget https://q4.github.io/bootgrub.gz
	gzip -d < bootgrub.gz | dd of=build/grub.img
	rm bootgrub.gz

build/floppy.img: build/grub.img kernel
	cp build/grub.img build/floppy.img
	mcopy -i build/floppy.img ./build/kernel ::/boot/kernel
	mdel -i build/floppy.img /boot/grub/menu.lst
	mcopy -i build/floppy.img ./grub.lst ::/boot/grub/menu.lst
