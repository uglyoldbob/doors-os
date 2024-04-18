all: qemucd

qemu32: build/floppy32.img
	qemu-system-i386 -fda build/floppy32.img -m 4 -d int,cpu_reset
qemu64: build/floppy64.img
	qemu-system-x86_64 -fda build/floppy64.img -m 4 -d int,cpu_reset

qemucd32: build/cd32.img
	qemu-system-i386 -cdrom build/cd32.img -m 4 -d int,cpu_reset

qemucd64: build/cd64.img
	qemu-system-x86_64 -cdrom build/cd64.img -m 4 -d int,cpu_reset

bochs64: build/cd64.img
	bochs -f bochsrc64.txt -q

bochs32: build/cd32.img
	bochs -f bochsrc32.txt -q

virtualbox32: build/cd32.img
	VirtualBoxVM --startvm test --dbg --debug

virtualbox64: build/cd64.img
	VirtualBoxVM --startvm test64 --dbg --debug

gdb64: build/cd64.img
	gdb -x script64.gdb

gdb32: build/cd32.img
	gdb -x script32.gdb

kernel64:
	mkdir -p ./build
	cargo build --release --target x86_64-unknown-none --bin kernel
	cp -u target/x86_64-unknown-none/release/kernel ./build/kernel64

kernel32:
	mkdir -p ./build
	cargo build --release --target i386-unknown-none.json --bin kernel
	cp -u target/i386-unknown-none/release/kernel ./build/kernel32

.PHONY: kernel32 kernel64

build/cd64.img: kernel64
	mkdir -p build/iso/boot/grub
	cp grub2.lst ./build/iso/boot/grub/grub.cfg
	cp ./build/kernel64 ./build/iso/boot/kernel
	grub-mkrescue -o ./build/cd64.img build/iso
	rm -rf ./build/iso

build/cd32.img: kernel32
	mkdir -p build/iso/boot/grub
	cp grub2.lst ./build/iso/boot/grub/grub.cfg
	cp ./build/kernel32 ./build/iso/boot/kernel
	grub-mkrescue -o ./build/cd32.img build/iso
	rm -rf ./build/iso

build/grub.img:
	wget https://q4.github.io/bootgrub.gz
	gzip -d < bootgrub.gz | dd of=build/grub.img
	rm bootgrub.gz

build/floppy64.img: build/grub.img kernel64
	cp build/grub.img build/floppy64.img
	mcopy -i build/floppy64.img ./build/kernel64 ::/boot/kernel
	mdel -i build/floppy64.img /boot/grub/menu.lst
	mcopy -i build/floppy64.img ./grub.lst ::/boot/grub/menu.lst
