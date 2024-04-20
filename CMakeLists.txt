cmake_minimum_required(VERSION 3.22)
project(doors-os)

add_custom_target(
    kernel64
    BYPRODUCTS ./kernel64.debug ./kernel64 ./target
    COMMAND cargo build --release --target x86_64-unknown-none
    COMMAND mkdir -p ./build
    COMMAND cp -u target/x86_64-unknown-none/release/kernel ./kernel64.debug
    COMMAND cp -u target/x86_64-unknown-none/release/kernel ./kernel64
    COMMAND strip ./kernel64
)

if (WIN32)
    set(VIRTUALBOX "C:\\Program Files\\Oracle\\VirtualBox\\VirtualBoxVM.exe")
    set(VIRTUALBOX_MANAGE "C:\\Program Files\\Oracle\\VirtualBox\\VBoxManage.exe")
    set(VIRTUALBOX_IMAGE "C:\\Program Files\\Oracle\\VirtualBox\\vbox-img.exe")
    set(QEMU_X64 "C:\\Program Files\\qemu\\qemu-system-x86_64.exe")
endif (WIN32)

add_custom_command(
    OUTPUT grub.img
    COMMAND wget https://q4.github.io/bootgrub.gz
    COMMAND gzip -d bootgrub.gz
    COMMAND mv bootgrub grub.img
)

add_custom_command(
    OUTPUT grub.iso
    COMMAND wget -O grub.iso https://downloads.sourceforge.net/project/supergrub2/2.04s1/super_grub2_disk_2.04s1/super_grub2_disk_hybrid_2.04s1.iso?ts=gAAAAABmIxykQIvf8AFH3CE0bEUQUBNP4PpQW1BRHmyAOCVACxNxztzuD62gDfz6xgQgXmK35mcPwtHUNc8NRCFpndsMPuNcjw%3D%3D&r=https%3A%2F%2Fsourceforge.net%2Fprojects%2Fsupergrub2%2Ffiles%2F2.04s1%2Fsuper_grub2_disk_2.04s1%2Fsuper_grub2_disk_hybrid_2.04s1.iso%2Fdownload
)

if (WIN32)
add_custom_target(
    image_cd64
    DEPENDS kernel64
    DEPENDS grub.iso
    BYPRODUCTS cd64.iso
    COMMAND "C:\\Program Files\\7-Zip\\7z.exe" x -y -o./build/grubiso grub.iso
    COMMAND rm -f ./build/grubiso/boot/grub/grub.cfg
    COMMAND cp grub2.lst ./build/grubiso/boot/grub/grub.cfg
    COMMAND cp ./build/kernel64 ./build/grubiso/boot/kernel
    COMMAND cp -f ./build/grubiso/efi.img build/grubiso/[boot]/efi.img
    COMMAND rm -f ./build/grubiso/efi.img
    COMMAND ${VIRTUALBOX_IMAGE} createiso -o cd64.iso --name-setup=iso9660 ./boot=build/grubiso/boot ./system=build/grubiso/system ./boot.catalog=build/grubiso/boot.catalog --eltorito-add-image=./build/grubiso/[boot]/efi.img --no-emulation-boot
    COMMAND ${VIRTUALBOX_IMAGE} createiso --import-iso grub.iso -o cd65.iso --name-setup=iso9660 ./boot/grub/menu.lst=./grub2.lst
)
endif (WIN32)

if (UNIX)
add_custom_target(
    image_cd64
    DEPENDS kernel64
    COMMAND mkdir -p build/iso/boot/grub
    COMMAND cp grub2.lst ./build/iso/boot/grub/grub.cfg
    COMMAND cp ./build/kernel64 ./build/iso/boot/kernel
    COMMAND grub-mkrescue -o ./build/cd64.iso build/iso
    COMMAND rm -rf ./build/iso
)
endif (UNIX)

add_custom_command(
    OUTPUT doors-os-64/doors-os-64.vbox
    DEPENDS image_cd64
    COMMAND ${VIRTUALBOX_MANAGE} createvm --name doors-os-64 --ostype "Doors" --register --basefolder `pwd`
    COMMAND ${VIRTUALBOX_MANAGE} storagectl doors-os-64 --name "IDE Controller" --add ide --controller PIIX4
    COMMAND ${VIRTUALBOX_MANAGE} storageattach doors-os-64 --storagectl "IDE Controller" --port 1 --device 0 --type dvddrive --medium ./cd64.iso
)

add_custom_target(
    virtualbox64
    DEPENDS doors-os-64/doors-os-64.vbox
    COMMAND ${VIRTUALBOX} --startvm doors-os-64 --dbg --debug
)

add_custom_target(
    qemucd64
    DEPENDS image_cd64
    COMMAND ${QEMU_X64} -cdrom cd64.iso -m 8 
)