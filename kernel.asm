[BITS 16]
jmp short Main			;don't execute data (execution begins at the beginning of the file, just like the bootsector)
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
mov si, Message
call ShowMessage

;enable pmode (steps)
;IDT
;-gate descriptors for all exceptions and interrupts (i plan to use interrupt/trap gates)
;GDT
;-two sets of code + data descriptors (user and kernel modes)
;TSS
;(LDT)
;page directory + page table
;code module to handle interrupt and exception handlers
;init the GDTR
;init CR1-CR3 cotrol registers (CR4 only with pentium +)
;enable the pmode bit in the CR0 control register
;IDTR to load the IDT
;load one page directory and one page table into RAM
;load PDBR (CR3) loaded with location of the page directory
;disable interrupts
;set the PG flag (CR0)
;far jump to 32 bit code
;(LLDT to load the LDTR register)
;LTR to load segment selector for TSS descriptor 
;load DS, SS, ES (null), FS (null), GS (null), 
;LIDT to load the IDTR for the pmode IDT
;enable interrupts

[BITS 32]

gdt:                    ; Address for the GDT
gdt_null:               ; Null Segment
        dd 0
        dd 0
gdt_code:               ; Code segment, execute, nonconforming
	dw 0xFFFF		;maximum size
	dw 0x0700		;begins at the beginning of this kernel
	db 0x00		;high of base is 0
	db 10001000b	;present, highest privelage, system descriptor, execute only
	db 11001111b	;granulated, 32-bit opcodes, segment length maximum
	db 0			;upper base is 0
gdt_data:               ; Data segment, read/write
	dw 0xFFFF		;maximum size
	dw 0x0700		;begin at the beginning of the kernel
	db 0x00		;high base = 0
	db 10000010b	;present, highest privelage, system descriptor, read/write data
	db 11001111b	;granulated, 32 bit operand size max segment size
	db 0			;high base = 0
gdt_screen:			;segment for the screen buffer (used to write text to the screen)
	dw 0x0F9F		;this should be eventually set to the low word of the size of the screen buffer (in bytes)
	dw 0x8000		;the beginning of the screen buffer
	db 0x0B		;the high byte of the beginning of the screen buffer
	db 10000010b	;present, highest privelage, system descriptor, read/write data
	db 01000000b	;no granularity, 32 bit operand size, highest nibble of segment size
	db 0			;highest byte of base address
gdt_end:
gdt_desc:                       ; The GDT descriptor
        dw gdt_end - gdt - 1    ; Limit (size)
        dd gdt                  ; Address of the GDT

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
	dw isr0 & 0x00FF	;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 10001110		;flags byte
	dw isr0 >> 16	;the upper byte of the offset
idt1:	;interrupt gate
	dw isr1 & 0x00FF	;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 10001110		;flags byte
	dw isr1 >> 16	;the upper byte of the offset
idt2:	;interrupt gate
	dw isr2 & 0x00FF	;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 10001110		;flags byte
	dw isr2 >> 16	;the upper byte of the offset
idt3:	;interrupt gate
	dw isr3 & 0x00FF	;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 10001110		;flags byte
	dw isr3 >> 16	;the upper byte of the offset
idt4:	;interrupt gate
	dw isr4 & 0x00FF	;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 10001110		;flags byte
	dw isr4 >> 16	;the upper byte of the offset
idt5:	;interrupt gate
	dw isr5 & 0x00FF	;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 10001110		;flags byte
	dw isr5 >> 16	;the upper byte of the offset
idt6:	;interrupt gate
	dw isr6 & 0x00FF	;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 10001110		;flags byte
	dw isr6 >> 16	;the upper byte of the offset
idt7:	;interrupt gate
	dw isr7 & 0x00FF	;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 10001110		;flags byte
	dw isr7 >> 16	;the upper byte of the offset
idt8:	;interrupt gate
	dw isr8 & 0x00FF	;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 10001110		;flags byte
	dw isr8 >> 16	;the upper byte of the offset


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
	mov byte [DS:0x50], bx
	mov ebx, eax
	shr ebx, 8
	shr ebx, 8		;the third byte in bx
	;adjust to 0-F
	add bx, 30h		;adjust to 0-...
	cmp bx, 0x3A	;if greater, add 
	jbe .Num2
	add bx, 0x07	;A-F
.Num2
	mov byte [DS:0x52], bx
	
	mov ebx, eax
	shr ebx, 8		;second byte in bx
	;adjust to 0-F
	add bx, 30h		;adjust to 0-...
	cmp bx, 0x3A	;if greater, add 
	jbe .Num3
	add bx, 0x07	;A-F
.Num3
	mov byte [DS:0x54], bx
	mov ebx, eax
	;we already have the low byte in bx
	;adjust to 0-F
	add bx, 30h		;adjust to 0-...
	cmp bx, 0x3A	;if greater, add 
	jbe .Num4
	add bx, 0x07	;A-F
.Num4
	mov byte [DS:0x56], bx
	pop es
	pop ds
	pop fs
	pop gs
	popa
	ret		;this is a routine, not an isr

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
