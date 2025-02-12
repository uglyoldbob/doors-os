add-symbol-file kernel64.debug

define exit
    monitor quit
    quit
end

break start64
break segment_not_present_asm
disp /i $pc
target remote | qemu-system-x86_64 -serial file:serial.log -cdrom cd64.iso -m 8 -gdb stdio -d cpu_reset -d int -monitor unix:/tmp/qemusock,server,nowait
#-monitor unix:/tmp/qemusock,server,nowait
#socat -,echo=0,icanon=0 unix-connect:/tmp/qemusock