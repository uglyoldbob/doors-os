    .section .data
    .global _start
    .global PAGE_DIRECTORY_BOOT1
    .extern GDT_TABLE_PTR
    .extern start64
    .extern MULTIBOOT2_DATA
    .align 8
    MULTIBOOT2_DATA: .quad 0
    .align 4096
    PAGE_TABLE_PML4_BOOT:
        .quad PAGE_TABLE_PDP_BOOT + 0x3
        .fill 511, 8, 0
    .align 4096
    PAGE_TABLE_PDP_BOOT:
        .quad PAGE_DIRECTORY_BOOT1 + 0x3
        .fill 511, 8, 0
    .align 4096
    PAGE_DIRECTORY_BOOT1:
        .quad 0x000000 + 0x83
        .quad 0x200000 + 0x83
        .fill 510, 8, 0
    .section .text
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
        or eax, 0xB0
        mov cr3, eax
        #enable long mode
        mov ecx, 0xc0000080
        #global descriptor table
        lgdt [GDT_TABLE_PTR]
        rdmsr
        or eax, 1<<8
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
        jmp prestart64
    .code64
    prestart64:
        mov [MULTIBOOT2_DATA], rbx
        jmp start64
    .loop:
        hlt
        jmp .loop