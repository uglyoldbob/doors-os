cmake_minimum_required(VERSION 3.22)
project(doors-os)

add_custom_target(
    kernel64
    BYPRODUCTS ./kernel64.debug ./kernel64 ./target ./kerneltest64
    COMMAND cargo build --release --target x86_64-unknown-none
    COMMAND mkdir -p ./build
    COMMAND cp -u target/x86_64-unknown-none/release/kernel ./kernel64.debug
    COMMAND cp -u target/x86_64-unknown-none/release/kernel ./kernel64
    COMMAND strip ./kernel64
    COMMAND cp -u target/x86_64-unknown-none/release/kerneltest ./kerneltest64
    COMMAND strip ./kerneltest64
)

add_custom_target(
    disassemble
    BYPRODUCTS ./disassemble.txt
    COMMAND cargo objdump --release --bin kernel -q -- -d > disassemble.txt
)

add_custom_target(
    symbols_bochs
    DEPENDS kernel64
    BYPRODUCTS ./symbols_bochs ./kernel64.sym
    COMMAND cargo objdump -q --bin kernel --release -- -t > ./objdump_output
    COMMAND tail -n +5 objdump_output > symbols_bochs
)

if (WIN32)
    set(VIRTUALBOX "C:\\Program Files\\Oracle\\VirtualBox\\VirtualBoxVM.exe")
    set(VIRTUALBOX_MANAGE "C:\\Program Files\\Oracle\\VirtualBox\\VBoxManage.exe")
    set(VIRTUALBOX_IMAGE "C:\\Program Files\\Oracle\\VirtualBox\\vbox-img.exe")
    set(QEMU_X64 "C:\\Program Files\\qemu\\qemu-system-x86_64.exe")
    set(BOCHS "C:\\Program Files\\Bochs-2.8\\bochs.exe")
endif (WIN32)
if (UNIX)
    set(VIRTUALBOX "VirtualBoxVM")
    set(VIRTUALBOX_MANAGE "VBoxManage")
    set(VIRTUALBOX_IMAGE "vbox-img")
    set(QEMU_X64 "qemu-system-x86_64")
    set(BOCHS "bochs")
endif (UNIX)

add_custom_command(
    OUTPUT grub.img
    COMMAND wget https://q4.github.io/bootgrub.gz
    COMMAND gzip -d bootgrub.gz
    COMMAND mv bootgrub grub.img
)

if (WIN32)
add_custom_target(
    image_cd64
    DEPENDS kernel64
    BYPRODUCTS cd64.iso cd64test.iso
    COMMAND ${VIRTUALBOX_IMAGE} createiso --import-iso grub-skeleton.iso -o cd64.iso --name-setup=iso9660 ./boot/kernel=./kernel64
    COMMAND ${VIRTUALBOX_IMAGE} createiso --import-iso grub-skeleton.iso -o cd64test.iso --name-setup=iso9660 ./boot/kernel=./kerneltest64
)

add_custom_target(
    bochs64
    DEPENDS image_cd64 symbols_bochs disassemble
    COMMAND ${BOCHS} -f bochsrc64_windows.txt -q
)
endif (WIN32)

if (UNIX)
add_custom_target(
    grub-skeleton
    BYPRODUCTS grub-skeleton.iso
    COMMAND mkdir -p build/iso/boot/grub
    COMMAND cp grub2.lst ./build/iso/boot/grub/grub.cfg
    COMMAND grub-mkrescue -o ./grub-skeleton.iso build/iso
    COMMAND rm -rf ./build/iso
)

add_custom_target(
    image_cd64
    DEPENDS kernel64
    BYPRODUCTS cd64.iso cd64test.iso
    COMMAND mkdir -p build/iso/boot/grub
    COMMAND cp grub2.lst ./build/iso/boot/grub/grub.cfg
    COMMAND cp ./kernel64 ./build/iso/boot/kernel
    COMMAND grub-mkrescue -o ./cd64.iso build/iso
    COMMAND cp ./kerneltest64 ./build/iso/boot/kernel
    COMMAND grub-mkrescue -o ./cd64test.iso build/iso
    COMMAND rm -rf ./build/iso
)

add_custom_target(
    bochs64
    DEPENDS image_cd64 symbols_bochs disassemble
    COMMAND ${BOCHS} -f bochsrc64.txt -q
)

add_custom_target(
    bochs64gdb
    DEPENDS image_cd64 symbols_bochs
    COMMAND ./bochs -f bochsrc64gdb.txt -q
)

add_custom_target(
    gdb64
    DEPENDS image_cd64
    COMMAND gdb -x script64.gdb
)
endif (UNIX)

add_custom_command(
    OUTPUT doors-os-64/doors-os-64.vbox
    DEPENDS image_cd64
    COMMAND ${VIRTUALBOX_MANAGE} createvm --name doors-os-64 --ostype "Doors" --register --basefolder `pwd`
    COMMAND ${VIRTUALBOX_MANAGE} modifyvm doors-os-64 --uart1 0x3f8 4 --uartmode1 file serial.log
    COMMAND ${VIRTUALBOX_MANAGE} modifyvm doors-os-64 --uart2 0x2f8 3 --uartmode2 file serial2.log
    COMMAND ${VIRTUALBOX_MANAGE} modifyvm doors-os-64 --nic1 nat
    COMMAND ${VIRTUALBOX_MANAGE} modifyvm doors-os-64 --nictype1 82540EM
    COMMAND ${VIRTUALBOX_MANAGE} storagectl doors-os-64 --name "IDE Controller" --add ide --controller PIIX4
    COMMAND ${VIRTUALBOX_MANAGE} storageattach doors-os-64 --storagectl "IDE Controller" --port 1 --device 0 --type dvddrive --medium ./cd64.iso
)

add_custom_target(
    virtualbox64debug
    DEPENDS doors-os-64/doors-os-64.vbox disassemble
    COMMAND ${VIRTUALBOX} --startvm doors-os-64 --dbg --debug
)

add_custom_target(
    virtualbox64
    DEPENDS doors-os-64/doors-os-64.vbox
    COMMAND ${VIRTUALBOX} --startvm doors-os-64
)

add_custom_target(
    qemucd64
    DEPENDS image_cd64
    COMMAND ${QEMU_X64} -cdrom cd64.iso -m 8 -serial file:serial.log -serial pty -netdev user,id=u1 -device e1000,netdev=u1 -object filter-dump,id=f1,netdev=u1,file=dump.dat -monitor telnet:127.0.0.1:55555,server,nowait;
)

add_custom_target(
    qemucd64test
    DEPENDS image_cd64
    COMMAND ${QEMU_X64} -cdrom cd64test.iso -m 8 -serial file:test.log -serial pty -netdev user,id=u1 -device e1000,netdev=u1 -object filter-dump,id=f1,netdev=u1,file=dump.dat
)