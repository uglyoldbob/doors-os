add-symbol-file ./kernel/kernel64.debug

define exit
    monitor quit
    quit
end

break kernel::boot::x86::boot64::irq0
break kernel::scheduler::Scheduler::handle_interrupt
break thread_restore
disp /i $pc
target remote | qemu-system-x86_64 -serial file:serial.log -serial file:serial2.log -cdrom cd64.iso -m 8 -gdb stdio

