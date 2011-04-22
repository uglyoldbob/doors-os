[ORG 0x0700]
TIMES 0x200 db '`'	;this will contain half of the protented mode stack
[BITS 32]
	mov ax, 0x10			;save data segment identifyer
	mov ss, ax				;set the stack segment for pmode
	mov esp, 0x08FF			;esp = 0x08FF
	mov ax, 0x10
	mov ds, ax
	mov byte [ds:0B8000h], 'P'      ; Move the ASCII-code of 'P' into first video memory
	mov byte [ds:0B8001h], 1Bh      ; Assign a color code
hang:
        jmp hang                ; Loop, self-jump








[BITS 16]
;this file is loaded to the following physical memory address - 0x0700
jmp Main			;don't execute data (execution begins at the beginning of the file, just like the bootsector)
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
;data goes here
Message db 'www.junkmachine.com', 13, 10, 0
Main:
;ds is already set
mov si, Message
call ShowMessage
cli
lgdt [gdt_desc - $$ + 0x0700]	;load the GDTR
mov eax, (pge_tbl_beg - $$ + 0x0700)
mov cr3, eax
mov eax, cr0
or eax, 1
mov cr0, eax	;enable pmode
jmp 08:Pmode
[BITS 32]
Pmode:
lidt [idt_desc - $$ + 0x0700];load the IDT
mov eax, 0x10
mov ds, eax
mov eax, 0x20
mov ss, eax
mov eax, 0x1400	;5 KB stack
mov esp, eax	;stack segment same size as original
mov eax, 0x18
mov es, eax		;for screen writing stuff
mov eax, cr0
and eax, 10000000000000000000000000000000b
mov cr0, eax	;enable paging
sti			;enable interrupts
mov [ES:0], BYTE 'Z'
jmp $
;enable pmode (steps)
;@IDT
;@-gate descriptors for all exceptions and interrupts (i plan to use interrupt/trap gates)
;@GDT
;@-two sets of code + data descriptors (user and kernel modes)
;(LDT)
;@page directory + page table
;@code module to handle interrupt and exception handlers
;@disable interrupts
;@init CR1-CR3 cotrol registers (CR4 only with pentium +)
;@init the GDTR
;@enable the pmode bit in the CR0 control register
;@far jump to 32 bit code
;@IDTR to load the IDT
;@load one page directory and one page table into RAM
;@load PDBR (CR3) loaded with location of the page directory
;@set the PG flag (CR0)
;(LLDT to load the LDTR register)
;@load DS, SS, ES (null), FS (null), GS (null), 
;@LIDT to load the IDTR for the pmode IDT
;*TSS
;*LTR to load segment selector for TSS descriptor 
;@enable interrupts

gdt:                    ; Address for the GDT
gdt_null:               ; Null Segment
        dd 0
        dd 0
gdt_code:               ; Code segment, execute, nonconforming 0x08
	dw 0xFFFF		;maximum size
	dw 0x0700		;begins at the beginning of this kernel
	db 0x00		;high of base is 0
	db 10011000b	;present, highest privelage, not system descriptor, execute only
	db 11001111b	;granulated, 32-bit opcodes, segment length maximum
	db 0x00			;upper base is 0
gdt_data:               ; Data segment, read/write 0x10
	dw 0xFFFF		;maximum size
	dw 0x0700		;begin at the beginning of the kernel
	db 0x00		;high base = 0
	db 10010010b	;present, highest privelage, not system descriptor, read/write data
	db 11001111b	;granulated, 32 bit operand size max segment size
	db 0			;high base = 0
gdt_screen:			;segment for the screen buffer (used to write text to the screen) 0x18
	dw 0x0F9F		;this should be eventually set to the low word of the size of the screen buffer (in bytes)
	dw 0x8000		;the beginning of the screen buffer
	db 0x0B		;the high byte of the beginning of the screen buffer
	db 10010010b	;present, highest privelage, not system descriptor, read/write data
	db 01000000B	;no granularity, 32 bit operand size, highest nibble of segment size
	db 0			;highest byte of base address
gdt_stack:			;0x20 (0x9FBFF)
	dw 0xE7FF		;1KB pmode stack
	dw 0xFBFF		;base address
	db 0x09		;base address
	db 10010110		;present, not system, read/write data, expand down
	db 01001001		;not granulated, 32-bit operands, 0x09 = high byte in limit
	db 0x00		;base address	
gdt_end:
gdt_desc:				; The GDT descriptor
        dw gdt_end-gdt-1	; Limit (size)
        dd gdt			; Address of the GDT


;TSS(s) goes here
;five data structures
;*Task State Segment
	;104 bytes long
	;dw prevTaskLink (segment selector for previous task, to allow switch back on an iret)
	;dw reserved 0
	;dd ESP for level 0 stack
	;dw SS for level 0 stack
	;dw reserved 0
	;dd ESP for level 1 stack
	;dw SS for level 1 stack
	;dw reserved 0
	;dd ESP for level 2 stack
	;dw SS for level 2 stack
	;dw reserved 0
	;dd cr3 (page-directory bas register)
	;dd eip
	;dd eflags
	;dd eax
	;dd ecx
	;dd edx
	;dd ebx
	;dd esp
	;dd ebp
	;dd esi
	;dd edi
	;dw es
	;dw reserved 0
	;dw cs
	;dw reserved 0
	;dw ss
	;dw reserved 0
	;dw ds
	;dw reserved 0
	;dw fs
	;dw reserved 0
	;dw gs
	;dw reserved 0
	;dw LDT segment selector
	;dw reserved 0
	;dw special (bit 0 is the debug trap flag, when set, a debug exception is raised when switched to)
	;dw I/O map base address (16-bit offset from base of TSS to I/O permission bit map and interrupt redirection bitmap)
		;this points to the beginning of the first and the end of the last
;*task-gate descriptor
	;provides indirect, protected reference to a task
	;can be placed in GDT, LDT, or IDT
	;dw reserved
	;dw TSS segment selector (points to a TSS descriptor in the GDT)
	;db reserved
	;db [P][DPL]00101	(Present, Descriptor Privelage Level)
	;dw reserved
;*TSS descriptor
	;goes in the GDT as a segment selector
	;points to a TSS (one per TSS)
	;dw lowerWordSegmentLimit
	;dw lowerWordBaseAddress
	;db thirdByteBaseAddress
	;db [P][DPL]010[B]1	(Present, Descriptor Privelage Level, Busy)
	;db [G]00[AVL], highNibbleLimit	(Granularity, AVaiLable)
	;db highByteBaseAddress
;*task register
	;16-bit segment selector (visible, points to a TSS descriptor in the GDT, and changed by str and ltr)
	;entire segment descriptor (32-bit base address, 16 bit limit, and attributes)
	 ;for the TSS of current task (copied from the TSS descriptor)
	;
;*NT flag in EFLAGS


;interrupt descriptor format
;interrupt gate
	;low word of handler offset
	;word segment selector
	;zero_byte				;low nibble is reserved
	;flag byte p dpl 0 1 1 1 0	;32 bits for size of gate
	;high word of handler offset

idt:				;address for the IDT
;10, 14 - use task gate

idt0:	;interrupt gate
	dw (isr0-$$) & 0x00FF	;get the lower word for the offset
	dw 0x08				;the code segment for our segment
	db 0					;a zero byte
	db 10001110				;flags byte
	dw (isr0-$$) >> 16			;the upper byte of the offset
idt1:	;interrupt gate
	dw (isr1-$$) & 0x00FF	;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 10001110		;flags byte
	dw (isr1-$$) >> 16	;the upper byte of the offset
idt2:	;interrupt gate
	dw (isr2-$$) & 0x00FF	;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 00001110		;flags byte (not present, supposedly reserved)
	dw (isr2-$$) >> 16	;the upper byte of the offset
idt3:	;interrupt gate
	dw (isr3-$$) & 0x00FF	;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 10001110		;flags byte
	dw (isr3-$$) >> 16	;the upper byte of the offset
idt4:	;interrupt gate
	dw (isr4-$$) & 0x00FF	;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 10001110		;flags byte
	dw (isr4-$$) >> 16	;the upper byte of the offset
idt5:	;interrupt gate
	dw (isr5-$$) & 0x00FF	;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 10001110		;flags byte
	dw (isr5-$$) >> 16	;the upper byte of the offset
idt6:	;interrupt gate
	dw (isr6-$$) & 0x00FF	;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 10001110		;flags byte
	dw (isr6-$$) >> 16	;the upper byte of the offset
idt7:	;interrupt gate
	dw (isr7-$$) & 0x00FF	;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 10001110		;flags byte
	dw (isr7-$$) >> 16	;the upper byte of the offset
idt8:	;interrupt gate
	dw (isr8-$$) & 0x00FF	;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 10001110		;flags byte
	dw (isr8-$$) >> 16	;the upper byte of the offset
idt9:	;interrupt gate
	dw (isr9-$$) & 0x00FF	;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 10001110		;flags byte
	dw (isr9-$$) >> 16	;the upper byte of the offset
idt10:	;interrupt gate
	dw (isr10-$$) & 0x00FF	;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 10001110		;flags byte
	dw (isr10-$$) >> 16	;the upper byte of the offset
idt11:	;interrupt gate
	dw (isr11-$$) & 0x00FF	;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 10001110		;flags byte
	dw (isr11-$$) >> 16	;the upper byte of the offset
idt12:	;interrupt gate
	dw (isr12-$$) & 0x00FF	;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 10001110		;flags byte
	dw (isr12-$$) >> 16	;the upper byte of the offset
idt13:	;interrupt gate
	dw (isr13-$$) & 0x00FF	;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 10001110		;flags byte
	dw (isr13-$$) >> 16	;the upper byte of the offset
idt14:	;interrupt gate
	dw (isr14-$$) & 0x00FF	;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 10001110		;flags byte
	dw (isr14-$$) >> 16	;the upper byte of the offset
idt15:	;interrupt gate
	dw (isr9-$$) & 0x00FF	;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 00001110		;flags byte (not present, reserved)
	dw (isr9-$$) >> 16	;the upper byte of the offset
idt16:	;interrupt gate
	dw (isr16-$$) & 0x00FF	;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 10001110		;flags byte
	dw (isr16-$$) >> 16	;the upper byte of the offset
idt17:	;interrupt gate
	dw (isr17-$$) & 0x00FF	;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 10001110		;flags byte
	dw (isr17-$$) >> 16	;the upper byte of the offset
idt18:	;interrupt gate
	dw (isr18-$$) & 0x00FF	;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 10001110		;flags byte
	dw (isr18-$$) >> 16	;the upper byte of the offset
idt19:	;interrupt gate
	dw (isr19-$$) & 0x00FF	;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 10001110		;flags byte
	dw (isr19-$$) >> 16	;the upper byte of the offset
;20 - 31 are reserved, 32-255 are usable for anything
idt_end:
idt_desc:				;IDT descriptor
	dw idt_end - idt - 1	;limit
	dd idt			;address for idt

;screen size = 80 x 25 (X 2)
Print:		;prints eax
;eax = the value to be printed
;prints to (ES:
	pusha
	push gs
	push fs
	push ds
	push es
	push eax	
	mov ax, 18h
	mov ds, ax		;write to the screen
	mov ebx, eax
	shr ebx, 8
	shr ebx, 8
	shr ebx, 8		;we want the high byte
	;adjust to 0-F
	add bx, 30h		;adjust to 0-...
	cmp bx, 0x3A	;if greater, add 
	jbe .Num1
	add bx, 0x07	;A-F
.Num1
        mov byte [DS:0x50], bl
	mov ebx, eax
	shr ebx, 8
	shr ebx, 8		;the third byte in bx
	;adjust to 0-F
	add bx, 30h		;adjust to 0-...
	cmp bx, 0x3A	;if greater, add 
	jbe .Num2
	add bx, 0x07	;A-F
.Num2
        mov byte [DS:0x52], bl
	
	mov ebx, eax
	shr ebx, 8		;second byte in bx
	;adjust to 0-F
	add bx, 30h		;adjust to 0-...
	cmp bx, 0x3A	;if greater, add 
	jbe .Num3
	add bx, 0x07	;A-F
.Num3
        mov byte [DS:0x54], bl
	mov ebx, eax
	;we already have the low byte in bx
	;adjust to 0-F
	add bx, 30h		;adjust to 0-...
	cmp bx, 0x3A	;if greater, add 
	jbe .Num4
	add bx, 0x07	;A-F
.Num4
        mov byte [DS:0x56], bl
	pop es
	pop ds
	pop fs
	pop gs
	popa
	ret		;this is a routine, not an (isr

Zero db 'D i v i d e   b y   z e r o   e r r o r ! ', 0
isr0:
;fault, no error code
;display message, stop, because returning brings it back
	mov ax, 18h
	mov es, ax			;the segment for the screen
	mov ax, 10h
	mov ds, ax			;data segment (we can use variables with their name now)
	xor ecx, ecx
	.loop				
	mov eax, Zero		;ecx = ofset into string (starts at 0), the character we are about to print
	add eax, ecx		;checks for null, but does not print the null
	mov edx, [eax]		;DS:EDX = the character we are about to print
	mov ebx, ecx
	cmp byte [DS:EDX], 0	;is it zero, if so, stop printing
	je .skip
	mov [ES:EBX], dx		;ES:EBX is where we are printing to
	inc cx
	jmp .loop
	.skip
	pop eax
	push eax			;retreive the EIP value, but leave it there
	call Print			;print eax
	cli				;disable interrupts
	jmp $				;hang
One db 'D e b u g   e x c e p t i o n ', 0
isr1:
;trap or fault, no error code
;examine DR6 and other debug registers to determine type (not necessary though)
	pusha
	push gs
	push fs
	push ds
	push es
	mov ax, 18h
	mov es, ax			;the segment for the screen
	mov ax, 10h
	mov ds, ax			;data segment (we can use variables with their name now)
	xor ecx, ecx
	.loop				
	mov eax, One		;ecx = ofset into string (starts at 0), the character we are about to print
	add eax, ecx		;checks for null, but does not print the null
	mov edx, [eax]		;DS:EDX = the character we are about to print
	mov ebx, ecx
	cmp byte [DS:EDX], 0	;is it zero, if so, stop printing
	je .skip
	mov [ES:EBX], dx		;ES:EBX is where we are printing to
	inc cx
	jmp .loop
	.skip
	pop eax
	push eax			;retreive the EIP value, but leave it there
	call Print			;print eax
	pop es
	pop ds
	pop fs
	pop gs
	popa
	iret
Two db 'N M I   I n t e r r u p t ', 0
isr2:
;interrupt, no error code
;hmm what to do? i dont know...
	pusha
	push gs
	push fs
	push ds
	push es
	mov ax, 18h
	mov es, ax			;the segment for the screen
	mov ax, 10h
	mov ds, ax			;data segment (we can use variables with their name now)
	xor ecx, ecx
	.loop				
	mov eax, Two		;ecx = ofset into string (starts at 0), the character we are about to print
	add eax, ecx		;checks for null, but does not print the null
	mov edx, [eax]		;DS:EDX = the character we are about to print
	mov ebx, ecx
	cmp byte [DS:EDX], 0	;is it zero, if so, stop printing
	je .skip
	mov [ES:EBX], dx		;ES:EBX is where we are printing to
	inc cx
	jmp .loop
	.skip
	pop eax
	push eax			;retreive the EIP value, but leave it there
	call Print			;print eax
	pop es
	pop ds
	pop fs
	pop gs
	popa
	iret
Three db 'B r e a k p o i n t ', 0
isr3:
;trap, no error code
	pusha
	push gs
	push fs
	push ds
	push es
	mov ax, 18h
	mov es, ax			;the segment for the screen
	mov ax, 10h
	mov ds, ax			;data segment (we can use variables with their name now)
	xor ecx, ecx
	.loop				
	mov eax, Three		;ecx = ofset into string (starts at 0), the character we are about to print
	add eax, ecx		;checks for null, but does not print the null
	mov edx, [eax]		;DS:EDX = the character we are about to print
	mov ebx, ecx
	cmp byte [DS:EDX], 0	;is it zero, if so, stop printing
	je .skip
	mov [ES:EBX], dx		;ES:EBX is where we are printing to
	inc cx
	jmp .loop
	.skip
	pop eax
	push eax			;retreive the EIP value, but leave it there
	call Print			;print eax
	pop es
	pop ds
	pop fs
	pop gs
	popa
	iret
Four db 'O v e r f l o w', 0
isr4:
;trap, no error code
	pusha
	push gs
	push fs
	push ds
	push es
	mov ax, 18h
	mov es, ax			;the segment for the screen
	mov ax, 10h
	mov ds, ax			;data segment (we can use variables with their name now)
	xor ecx, ecx
	.loop				
	mov eax, Four		;ecx = ofset into string (starts at 0), the character we are about to print
	add eax, ecx		;checks for null, but does not print the null
	mov edx, [eax]		;DS:EDX = the character we are about to print
	mov ebx, ecx
	cmp byte [DS:EDX], 0	;is it zero, if so, stop printing
	je .skip
	mov [ES:EBX], dx		;ES:EBX is where we are printing to
	inc cx
	jmp .loop
	.skip
	pop eax
	push eax			;retreive the EIP value, but leave it there
	call Print			;print eax
	pop es
	pop ds
	pop fs
	pop gs
	popa
	iret
Five db 'B o u n d s   r a n g e   e x c e e d e d', 0
isr5:
;fault, no error code
	pusha
	push gs
	push fs
	push ds
	push es
	mov ax, 18h
	mov es, ax			;the segment for the screen
	mov ax, 10h
	mov ds, ax			;data segment (we can use variables with their name now)
	xor ecx, ecx
	.loop				
	mov eax, Five		;ecx = ofset into string (starts at 0), the character we are about to print
	add eax, ecx		;checks for null, but does not print the null
	mov edx, [eax]		;DS:EDX = the character we are about to print
	mov ebx, ecx
	cmp byte [DS:EDX], 0	;is it zero, if so, stop printing
	je .skip
	mov [ES:EBX], dx		;ES:EBX is where we are printing to
	inc cx
	jmp .loop
	.skip
	pop eax
	push eax			;retreive the EIP value, but leave it there
	call Print			;print eax
	pop es
	pop ds
	pop fs
	pop gs
	popa
	iret
Six db 'I n v a l i d   o p c o d e ', 0
isr6:
;fault, no error code
	pusha
	push gs
	push fs
	push ds
	push es
	mov ax, 18h
	mov es, ax			;the segment for the screen
	mov ax, 10h
	mov ds, ax			;data segment (we can use variables with their name now)
	xor ecx, ecx
	.loop				
	mov eax, Six		;ecx = ofset into string (starts at 0), the character we are about to print
	add eax, ecx		;checks for null, but does not print the null
	mov edx, [eax]		;DS:EDX = the character we are about to print
	mov ebx, ecx
	cmp byte [DS:EDX], 0	;is it zero, if so, stop printing
	je .skip
	mov [ES:EBX], dx		;ES:EBX is where we are printing to
	inc cx
	jmp .loop
	.skip
	pop eax
	push eax			;retreive the EIP value, but leave it there
	call Print			;print eax
	pop es
	pop ds
	pop fs
	pop gs
	popa
	iret
Seven db 'D e v i c e   n o t   a v a i l a b l e ', 0
isr7:
;fault, no error code
;this is confusing
	pusha
	push gs
	push fs
	push ds
	push es
	mov ax, 18h
	mov es, ax			;the segment for the screen
	mov ax, 10h
	mov ds, ax			;data segment (we can use variables with their name now)
	xor ecx, ecx
	.loop				
	mov eax, Seven		;ecx = ofset into string (starts at 0), the character we are about to print
	add eax, ecx		;checks for null, but does not print the null
	mov edx, [eax]		;DS:EDX = the character we are about to print
	mov ebx, ecx
	cmp byte [DS:EDX], 0	;is it zero, if so, stop printing
	je .skip
	mov [ES:EBX], dx		;ES:EBX is where we are printing to
	inc cx
	jmp .loop
	.skip
	pop eax
	push eax			;retreive the EIP value, but leave it there
	call Print			;print eax
	pop es
	pop ds
	pop fs
	pop gs
	popa
	iret
Eight db 'D o u b l e   -   f a u l t ', 0
isr8:
;abort, error code does exist (it is zero)
;there is no return from here, the program must be closed, and data logged
	mov ax, 18h
	mov es, ax			;the segment for the screen
	mov ax, 10h
	mov ds, ax			;data segment (we can use variables with their name now)
	xor ecx, ecx
	.loop				
	mov eax, Eight		;ecx = ofset into string (starts at 0), the character we are about to print
	add eax, ecx		;checks for null, but does not print the null
	mov edx, [eax]		;DS:EDX = the character we are about to print
	mov ebx, ecx
	cmp byte [DS:EDX], 0	;is it zero, if so, stop printing
	je .skip
	mov [ES:EBX], dx		;ES:EBX is where we are printing to
	inc cx
	jmp .loop
	.skip
	pop eax
	call Print			;we dont need to keep the error code on the stack
	jmp $				;we can't iret to the process, so hang yourself
Nine db 'C o p r o c e s s o r   s e g m e n t   o v e r r u n ', 0
isr9:
;abort, no error code
;FPU must be restarted (so we won't return for now)
	mov ax, 18h
	mov es, ax			;the segment for the screen
	mov ax, 10h
	mov ds, ax			;data segment (we can use variables with their name now)
	xor ecx, ecx
	.loop				
	mov eax, Nine		;ecx = ofset into string (starts at 0), the character we are about to print
	add eax, ecx		;checks for null, but does not print the null
	mov edx, [eax]		;DS:EDX = the character we are about to print
	mov ebx, ecx
	cmp byte [DS:EDX], 0	;is it zero, if so, stop printing
	je .skip
	mov [ES:EBX], dx		;ES:EBX is where we are printing to
	inc cx
	jmp .loop
	.skip
	pop eax
	push eax			;retreive the EIP value, then put it back
	call Print			;print eax
	jmp $				;freeze up this is an abort we dont know what to do
Ten db 'I n v a l i d   T S S   e x c e p t i o n', 0
isr10:
;fault, error code present
;must use a task gate, to preserve stability
	mov ax, 18h
	mov es, ax			;the segment for the screen
	mov ax, 10h
	mov ds, ax			;data segment (we can use variables with their name now)
	xor ecx, ecx
	.loop				
	mov eax, Ten		;ecx = ofset into string (starts at 0), the character we are about to print
	add eax, ecx		;checks for null, but does not print the null
	mov edx, [eax]		;DS:EDX = the character we are about to print
	mov ebx, ecx
	cmp byte [DS:EDX], 0	;is it zero, if so, stop printing
	je .skip
	mov [ES:EBX], dx		;ES:EBX is where we are printing to
	inc cx
	jmp .loop
	.skip
	pop eax
	push eax			;retreive the EIP value, then put it back
	call Print			;print eax
	jmp $				;freeze up this is an abort we dont know what to do
Eleven db 'S e g m e n t   n o t   p r e s e n t ', 0
isr11:
;fault, error code present
	mov ax, 18h
	mov es, ax			;the segment for the screen
	mov ax, 10h
	mov ds, ax			;data segment (we can use variables with their name now)
	xor ecx, ecx
	.loop				
	mov eax, Eleven		;ecx = ofset into string (starts at 0), the character we are about to print
	add eax, ecx		;checks for null, but does not print the null
	mov edx, [eax]		;DS:EDX = the character we are about to print
	mov ebx, ecx
	cmp byte [DS:EDX], 0	;is it zero, if so, stop printing
	je .skip
	mov [ES:EBX], dx		;ES:EBX is where we are printing to
	inc cx
	jmp .loop
	.skip
	pop eax			;retreive the error value, we dont need to put it back
	call Print			;print eax
	jmp $				;freeze up this is an abort we dont know what to do
Twelve db 'S t a c k   f a u l t   e x c e p t i o n', 0
isr12:
;fault, error code present
	mov ax, 18h
	mov es, ax			;the segment for the screen
	mov ax, 10h
	mov ds, ax			;data segment (we can use variables with their name now)
	xor ecx, ecx
	.loop				
	mov eax, Twelve		;ecx = ofset into string (starts at 0), the character we are about to print
	add eax, ecx		;checks for null, but does not print the null
	mov edx, [eax]		;DS:EDX = the character we are about to print
	mov ebx, ecx
	cmp byte [DS:EDX], 0	;is it zero, if so, stop printing
	je .skip
	mov [ES:EBX], dx		;ES:EBX is where we are printing to
	inc cx
	jmp .loop
	.skip
	pop eax			;retreive the error value, we dont need to put it back
	call Print			;print eax
	jmp $				;freeze up this is an abort we dont know what to do
Thirteen db 'G e n e r a l   P r o t e c t i o n   F a u l t ', 0
isr13:
;fault, error code present
	mov ax, 18h
	mov es, ax			;the segment for the screen
	mov ax, 10h
	mov ds, ax			;data segment (we can use variables with their name now)
	xor ecx, ecx
	.loop				
	mov eax, Thirteen		;ecx = ofset into string (starts at 0), the character we are about to print
	add eax, ecx		;checks for null, but does not print the null
	mov edx, [eax]		;DS:EDX = the character we are about to print
	mov ebx, ecx
	cmp byte [DS:EDX], 0	;is it zero, if so, stop printing
	je .skip
	mov [ES:EBX], dx		;ES:EBX is where we are printing to
	inc cx
	jmp .loop
	.skip
	pop eax			;retreive the error value, we dont need to put it back
	call Print			;print eax
	jmp $				;freeze up this is an abort we dont know what to do
Fourteen db 'P a g e   F a u l t   E x c e p t i o n ', 0
isr14:
;fault, special error code format same size though
;call through task gate, to allow page faulting during task switches
	mov ax, 18h
	mov es, ax			;the segment for the screen
	mov ax, 10h
	mov ds, ax			;data segment (we can use variables with their name now)
	xor ecx, ecx
	.loop				
	mov eax, Fourteen		;ecx = ofset into string (starts at 0), the character we are about to print
	add eax, ecx		;checks for null, but does not print the null
	mov edx, [eax]		;DS:EDX = the character we are about to print
	mov ebx, ecx
	cmp byte [DS:EDX], 0	;is it zero, if so, stop printing
	je .skip
	mov [ES:EBX], dx		;ES:EBX is where we are printing to
	inc cx
	jmp .loop
	.skip
	pop eax			;retreive the error value, we dont need to put it back
	call Print			;print eax
	jmp $				;freeze up this is an abort we dont know what to do
Sixteen db 'x 8 7   F P U   F l o a t i n g - P o i n t   E r r o r ', 0
isr16:
;fault, no error code
	mov ax, 18h
	mov es, ax			;the segment for the screen
	mov ax, 10h
	mov ds, ax			;data segment (we can use variables with their name now)
	xor ecx, ecx
	.loop				
	mov eax, Sixteen		;ecx = ofset into string (starts at 0), the character we are about to print
	add eax, ecx		;checks for null, but does not print the null
	mov edx, [eax]		;DS:EDX = the character we are about to print
	mov ebx, ecx
	cmp byte [DS:EDX], 0	;is it zero, if so, stop printing
	je .skip
	mov [ES:EBX], dx		;ES:EBX is where we are printing to
	inc cx
	jmp .loop
	.skip
	pop eax
	push eax			;retreive the EIP value, then put it back
	call Print			;print eax
	jmp $				;freeze up this is an abort we dont know what to do
Seventeen db 'A l i g n m e n t   C h e c k   E x c e p t i o n ', 0
isr17:
;fault, error code present
	mov ax, 18h
	mov es, ax			;the segment for the screen
	mov ax, 10h
	mov ds, ax			;data segment (we can use variables with their name now)
	xor ecx, ecx
	.loop				
	mov eax, Seventeen	;ecx = ofset into string (starts at 0), the character we are about to print
	add eax, ecx		;checks for null, but does not print the null
	mov edx, [eax]		;DS:EDX = the character we are about to print
	mov ebx, ecx
	cmp byte [DS:EDX], 0	;is it zero, if so, stop printing
	je .skip
	mov [ES:EBX], dx		;ES:EBX is where we are printing to
	inc cx
	jmp .loop
	.skip
	pop eax			;retreive the error value, we dont need to put it back
	call Print			;print eax
	jmp $				;freeze up this is an abort we dont know what to do
Eighteen db 'M a c h i n e - C h e c k   E x c e p t i o n', 0
isr18:
;abort, no error code
	mov ax, 18h
	mov es, ax			;the segment for the screen
	mov ax, 10h
	mov ds, ax			;data segment (we can use variables with their name now)
	xor ecx, ecx
	.loop				
	mov eax, Eighteen		;ecx = ofset into string (starts at 0), the character we are about to print
	add eax, ecx		;checks for null, but does not print the null
	mov edx, [eax]		;DS:EDX = the character we are about to print
	mov ebx, ecx
	cmp byte [DS:EDX], 0	;is it zero, if so, stop printing
	je .skip
	mov [ES:EBX], dx		;ES:EBX is where we are printing to
	inc cx
	jmp .loop
	.skip
	pop eax
	push eax			;retreive the EIP value, then put it back
	call Print			;print eax
	jmp $				;freeze up this is an abort we dont know what to do
Nineteen db 'S I M D   F l o a t i n g - P o i n t   E x c e p t i o n ', 0
isr19:
;fault, no error code
	mov ax, 18h
	mov es, ax			;the segment for the screen
	mov ax, 10h
	mov ds, ax			;data segment (we can use variables with their name now)
	xor ecx, ecx
	.loop				
	mov eax, Nineteen		;ecx = ofset into string (starts at 0), the character we are about to print
	add eax, ecx		;checks for null, but does not print the null
	mov edx, [eax]		;DS:EDX = the character we are about to print
	mov ebx, ecx
	cmp byte [DS:EDX], 0	;is it zero, if so, stop printing
	je .skip
	mov [ES:EBX], dx		;ES:EBX is where we are printing to
	inc cx
	jmp .loop
	.skip
	pop eax
	push eax			;retreive the EIP value, then put it back
	call Print			;print eax
	jmp $				;freeze up this is an abort we dont know what to do


;for this version of Doors, we will be using 4-KByte pages, and be able to access 4 GB or memory
;page directory beginning
;needs to be aligned to a 4KB boundary
;align 4096, db '~'
;TIMES (4096*2)-($-$$+0x0700) db '~'

pge_dir_beg:
dd ((pge_tbl_beg - $$ + 0x0700) << 12) + 000000000011

pge_tbl_beg:	;256 of these (1 KB total in size)
			;start at 0, inc by 4096d, 1000h, 1000000000000b
			;there isenough here to map 1MB of memory
;dd ????????????????????XXXZZZZZZZZZ
%assign i 0 
%rep    256 
dd 00000000000000000000000000000011b + i * 1000000000000000000000000b
	;present, supervisor, read/write, no caching,
	;adds in the memory base address for the pages
%assign i i+1 
%endrep
