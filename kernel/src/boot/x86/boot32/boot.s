    .section .data
    .global _start
    .global MULTIBOOT2_DATA
    .global INITIAL_STACK
    .extern GDT_TABLE_PTR
    .extern start32
    .align 8
    MULTIBOOT2_DATA: .4byte 0
    INITIAL_STACK: .4byte 0
    .align 4096
    PAGE_DIRECTORY:
        .4byte 0x000000 + 0x83
        .fill 1023, 4, 0
    .section .text
    .code32
    _start:
        mov al, 'A'
        mov [0xb8000], al
        #disable paging
        mov eax, cr0
        and eax, 0xEFFFFFFF
        mov cr0, eax
        #enable 4 mbyte pages
        mov eax, cr4
        or eax, 0x10
        mov cr4, eax
        #load cr3 with base of PML4
        lea eax, [PAGE_DIRECTORY]
        mov cr3, eax
        #global descriptor table
        lgdt [GDT_TABLE_PTR]
        #enable paging
        mov eax, cr0
        or eax, 0xE0000001
        mov cr0, eax
        mov eax, 8
        push eax
        lea eax, enter_regular
        push eax
        retf
    .code32
    enter_regular:
        mov ax, 0x10
        mov ds, ax
        mov es, ax
        mov fs, ax
        mov gs, ax
        mov ss, ax
        mov eax, 8
        push eax
        lea eax, prestart32
        push eax
        retf
    .code32
    prestart32:
        mov [MULTIBOOT2_DATA], ebx
        mov [INITIAL_STACK], esp
        jmp start32