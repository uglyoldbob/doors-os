add-symbol-file ./kernel/kernel64.debug
disp /i $pc
#set debug remote 1
set remotetimeout 30
target remote :1234
