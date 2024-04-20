cmake_minimum_required(VERSION 3.22)
project(doors-os)

add_custom_target(
    kernel64
    BYPRODUCTS ./build/kernel64.debug ./build/kernel64 ./target
    COMMAND cargo build --release --target x86_64-unknown-none
    COMMAND mkdir -p ./build
    COMMAND cp -u target/x86_64-unknown-none/release/kernel ./build/kernel64.debug
    COMMAND cp -u target/x86_64-unknown-none/release/kernel ./build/kernel64
    COMMAND strip ./build/kernel64
)

add_custom_target(
    qemucd64
    DEPENDS kernel64
    COMMAND qemu-system-x86_64 -cdrom build/cd64.iso -m 8 -d int,cpu_reset
)

if (WIN32)
    set(VIRTUALBOX "C:\\Program Files\\Oracle\\VirtualBox\\VirtualBoxVM.exe")
    set(VIRTUALBOX_MANAGE "C:\\Program Files\\Oracle\\VirtualBox\\VBoxManage.exe")
    set(VIRTUALBOX_IMAGE "C:\\Program Files\\Oracle\\VirtualBox\\vbox-img.exe")
endif (WIN32)

if (WIN32)
add_custom_target(
    image_cd64
    DEPENDS kernel64
    BYPRODUCTS ./build/image_cd64.iso
    COMMAND mkdir -p build/iso/boot/grub
    COMMAND cp grub2.lst ./build/iso/boot/grub/grub.cfg
    COMMAND cp ./build/kernel64 ./build/iso/boot/kernel
    COMMAND ${VIRTUALBOX_IMAGE} createiso -o `pwd`/build/cd64.iso --name-setup=iso9660 /=./build/iso
    COMMAND rm -rf ./build/iso
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
    COMMAND ${VIRTUALBOX_MANAGE} storageattach doors-os-64 --storagectl "IDE Controller" --port 1 --device 0 --type dvddrive --medium ./build/cd64.iso
)

add_custom_target(
    virtualbox64
    DEPENDS doors-os-64/doors-os-64.vbox
    COMMAND ${VIRTUALBOX} --startvm doors-os-64 --dbg --debug
)