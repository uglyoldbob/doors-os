add-symbol-file ./kernel/kernel64.debug
set architecture i386:x86-64
disp /i $pc
set debug remote 1
target remote :1234
