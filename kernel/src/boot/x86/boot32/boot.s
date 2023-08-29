    .section .data
    .global start
    .extern MULTIBOOT2_DATA
    .extern start32
    .align 8
    MULTIBOOT2_DATA: .word 0
    .section .text
    .code32
    start:
        mov [MULTIBOOT2_DATA], ebx
        jmp start32