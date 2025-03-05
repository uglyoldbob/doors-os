    .section .data
    .global _start
    .global MULTIBOOT2_DATA
    .global INITIAL_STACK
    .extern start32
    .align 8
    MULTIBOOT2_DATA: .4byte 0
    INITIAL_STACK: .4byte 0
    .section .text
    .code32
    _start:
        mov [MULTIBOOT2_DATA], ebx
        mov [INITIAL_STACK], esp
        jmp start32