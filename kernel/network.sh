cp src/kernel kernel.bin
strip kernel.bin
sudo cp kernel.bin /tftpboot/kernel.bin
cp src/kernel kernel.bin
sudo chown -R nobody:nogroup /tftpboot
sudo chmod -R 555 /tftpboot
qemu -boot n -net nic -net tap 

#-tftp /tftpboot -bootp /ns8390/pxegrub
#cp kernel.bin /tftpboot/kernel.bin
#gdb -s kernel.bin -ex 'target remote /dev/ttyS0'

