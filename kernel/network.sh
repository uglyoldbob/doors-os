cp kernel.bin /tftpboot/kernel.bin
gdb -s kernel.bin -ex 'target remote /dev/ttyS0'

