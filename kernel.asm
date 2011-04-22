[BITS 16]
jmp short Main			;don't execute data (execution begins at the beginning of the file, just like the bootsector)

;ata goes here
Message db 'This is a test, this is just a test...', 13, 10, 0
Main:
mov si, Message
call ShowMessage

;create a valid Global Descriptor Table (GDT), 
;(optional) create a valid Interrupt Descriptor Table (IDT), 
;disable interrupts, 
;point GDTR to your GDT, 
;(optional) point IDTR to your IDT, 
;set the PE bit in the MSW register, 
;do a far jump (load both CS and IP/EIP) to enter protected mode (load CS with the code segment selector), 
;load the DS and SS registers with the data/stack segment selector, 
;set up a pmode stack, 
;(optional) enable interrupts.

cli			;disable interrupts

xor ax, ax
mov ds, ax		;for the lgdt command
lgdt [gdt_desc]

;enter pmode

mov eax, cr0
or eax, 1
mov cr0, eax

;clear pipe

jmp 08h:Pmode
[BITS 32]

Pmode:

;load remaining registers with pmode values

mov ax, 0x10
mov ds, ax
mov ss, ax
mov esp, 090000h ; Move the stack pointer to 090000h 

sti			;enable interrupts (we have already set up our stack)

mov [0xb8020], BYTE 'P'
mov [0xb8021], BYTE 1Bh
hang:
	jmp hang

;set up GDT


gdt:                    ; Address for the GDT

gdt_null:               ; Null Segment
        dd 0
        dd 0

gdt_code:               ; Code segment, read/execute, nonconforming
        dw 0FFFFh
        dw 0
        db 0
        db 10011010b
        db 11001111b
        db 0

gdt_data:               ; Data segment, read/write, expand down
        dw 0FFFFh
        dw 0
        db 0
        db 10010010b
        db 11001111b
        db 0

gdt_end:                ; Used to calculate the size of the GDT



gdt_desc:                       ; The GDT descriptor
        dw gdt_end - gdt - 1    ; Limit (size)
        dd gdt                  ; Address of the GDT

[BITS 16]

;16 bit routines

ShowMessage:
	lodsb                                       ; load next character
	or      al, al                              ; test for NUL character
	jz      .DONE
	mov     ah, 0x0E                            ; BIOS teletype
	mov     bh, 0x00                            ; display page 0
	mov     bl, 0x07                            ; text attribute
	int     0x10                                ; invoke BIOS
	jmp     ShowMessage
.DONE:
	ret