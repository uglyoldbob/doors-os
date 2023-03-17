    .section .text
    .global _start
    .extern GDT_TABLE_PTR
    .extern start64
    .align 4096
    PAGE_TABLE_PML4_BOOT:
        .quad PAGE_TABLE_PDP_BOOT + 3
        .quad 0
        .fill 510, 8, 0
    PAGE_TABLE_PDP_BOOT:
        .quad 0x83
        .fill 511, 8, 0
    PAGE_TABLE_DIRECTORY_BOOT:
        .quad 0x83
        .fill 511, 8, 0
    .code32
    _start:
        mov al, 'A'
        mov [0xb8000], al
        #disable paging
        mov eax, cr0
        and eax, 0xEFFFFFFF
        mov cr0, eax
        #enable physical address extensions
        mov eax, cr4
        or eax, 0x20
        mov cr4, eax
        #load cr3 with base of PML4
        lea eax, [PAGE_TABLE_PML4_BOOT]
        or eax, 0x18
        mov cr3, eax
        #enable long mode
        mov ecx, 0xc0000080
        #global descriptor table
        lgdt [GDT_TABLE_PTR]
        rdmsr
        mov eax, 1<<10 | 1<<8
        wrmsr
        #enable paging
        mov eax, cr0
        or eax, 0xE0000001
        mov cr0, eax
        mov eax, 8
        push eax
        lea eax, enter_long
        push eax
        retf
    .code32
    enter_long:
        mov ax, 0x8
        nop
        mov ds, ax
        mov es, ax
        mov fs, ax
        mov gs, ax
        mov eax, 0x10
        mov ss, ax
        jmp start64
    .loop:
        hlt
        jmp .loop