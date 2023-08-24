add-symbol-file build/kernel64
break start64
break segment_not_present_asm
disp /i $pc
target remote | qemu-system-x86_64 -cdrom build/cd64.img -m 4 -gdb stdio -d cpu_reset -d int -monitor unix:/tmp/qemusock,server,nowait
#-monitor unix:/tmp/qemusock,server,nowait
#socat -,echo=0,icanon=0 unix-connect:/tmp/qemusock