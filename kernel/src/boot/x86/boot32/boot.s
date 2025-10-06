    .section .data
    .global _start
    .global MULTIBOOT2_DATA
    .global TABLE3
    .global TABLE2
    .global TABLE1
    .global INITIAL_STACK
    .extern GDT_TABLE_PTR
    .extern start32
    .align 8
    MULTIBOOT2_DATA: .4byte 0
    INITIAL_STACK: .4byte 0
    .align 4096
    PDPT:
        .fill 512, 8, 0
    .align 4096
    PAGE_DIRECTORY:
        .quad 0x000000 + 0x83
        .quad 0x200000 + 0x83
        .fill 510, 8, 0
    .align 4096
    PAGE_TABLE:
        TABLE3:
            .quad 0
        TABLE2:
            .quad 0
        TABLE1:
            .quad 0
        .fill 509, 8, 0
    .section .text
    .code32
    _start:
        mov al, 'A'
        mov [0xb8000], al
        #disable paging
        mov eax, cr0
        and eax, 0xEFFFFFFF
        mov cr0, eax
        #enable 4 mbyte pages, and pae
        mov eax, cr4
        or eax, 0x30
        mov cr4, eax
        #fill out the pdpt entry TODO
        lea eax, [PAGE_DIRECTORY]
        or eax, 1
        mov [PDPT], eax
        lea eax, [PAGE_TABLE]
        or eax, 3
        mov [PAGE_DIRECTORY + 16], eax
        #load cr3 with base of pdpt
        lea eax, [PDPT]
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