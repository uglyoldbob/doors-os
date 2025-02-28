add-symbol-file ./kernel/kernel64.debug

define exit
    monitor quit
    quit
end

disp /i $pc
target remote | qemu-system-x86_64 -serial file:serial.log -serial tcp::1234,server,nowait,nodelay -cdrom cd64.iso -m 8 -gdb stdio

