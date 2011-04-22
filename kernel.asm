[BITS 32]
[global start] 
[extern _main]	;this is in our C++ code
[extern __main]	;this is in our C support code
[extern __atexit] ;this is in our C support code
[extern __Z7displayPc]	;this is our display(char*) function

	mov ax, 0x10
	mov ds, ax			;set the segment registers
	mov ss, eax			;and stack
	xor eax, eax
	mov es, eax
	mov fs, eax
	mov gs, eax
	lgdt [gdt_desc + 0x0900]
		;this is so the place where the GDT used to be wont be a problem when the stack overwrites it
	mov eax, 0x0900
	mov esp, eax
	lidt [idt_desc]	;load the IDT
	sti			;enable interrupts
	start:
	call __main
	call _main 		;call int main(void), which is located in our C++ code
	call __atexit
	cli
	hlt 			;halt the CPU
gdt:                    ; Address for the GDT
gdt_null:               ; Null Segment
	dd 0
	dd 0
gdt_code:               ; Code segment, read/execute, nonconforming
	dw 0FFFFh
	dw 0x0000
	db 0
	db 10011010b
	db 11001111b
	db 0
gdt_data:               ; Data segment, read/write
	dw 0xFFFF
	dw 0x0000
	db 0
	db 10010010b
	db 11001111b
	db 0
gdt_end:				; Used to calculate the size of the GDT
gdt_desc:				; The GDT descriptor
	dw gdt_end - gdt - 1	; Limit (size)
	dd gdt + 0x0900		; Address of the GDT

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
      dw (isr0+0x0900-$$-2) & 0xFFFF  ;get the lower word for the offset
	dw 0x08				;the code segment for our segment
	db 0
	db 10001110b
	dw (isr0+0x0900-$$) >> 16	;the upper byte of the offset
idt1:	;interrupt gate
	dw (isr1-$$+0x0900-2) & 0xFFFF	;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 10001110b	;flags byte
	dw (isr1-$$+0x0900) >> 16	;the upper byte of the offset
idt2:	;interrupt gate
	dw (isr2-$$+0x0900-2) & 0xFFFF	;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 00001110b	;flags byte (not present, supposedly reserved)
	dw (isr2-$$+0x0900) >> 16	;the upper byte of the offset
idt3:	;interrupt gate
	dw (isr3-$$+0x0900-2) & 0xFFFF	;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 10001110b	;flags byte
	dw (isr3-$$+0x0900) >> 16	;the upper byte of the offset
idt4:	;interrupt gate
	dw (isr4-$$+0x0900-2) & 0xFFFF	;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 10001110b	;flags byte
	dw (isr4-$$+0x0900) >> 16	;the upper byte of the offset
idt5:	;interrupt gate
	dw (isr5-$$+0x0900-2) & 0xFFFF	;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 10001110b	;flags byte
	dw (isr5-$$+0x0900) >> 16	;the upper byte of the offset
idt6:	;interrupt gate
	dw (isr6-$$+0x0900-2) & 0xFFFF	;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 10001110b	;flags byte
	dw (isr6-$$+0x0900) >> 16	;the upper byte of the offset
idt7:	;interrupt gate
	dw (isr7-$$+0x0900-2) & 0xFFFF	;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 10001110b	;flags byte
	dw (isr7-$$+0x0900) >> 16	;the upper byte of the offset
idt8:	;interrupt gate
	dw (isr8-$$+0x0900-2) & 0xFFFF	;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 10001110b	;flags byte
	dw (isr8-$$+0x0900) >> 16	;the upper byte of the offset
idt9:	;interrupt gate
	dw (isr9-$$+0x0900-2) & 0xFFFF	;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 10001110b	;flags byte
	dw (isr9-$$+0x0900) >> 16	;the upper byte of the offset
idt10:	;interrupt gate
	dw (isr10-$$+0x0900-2) & 0xFFFF	;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 10001110b	;flags byte
	dw (isr10-$$+0x0900) >> 16	;the upper byte of the offset
idt11:	;interrupt gate
	dw (isr11-$$+0x0900-2) & 0xFFFF	;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 10001110b	;flags byte
	dw (isr11-$$+0x0900) >> 16	;the upper byte of the offset
idt12:	;interrupt gate
	dw (isr12-$$+0x0900-2) & 0xFFFF	;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 10001110b	;flags byte
	dw (isr12-$$+0x0900) >> 16	;the upper byte of the offset
idt13:	;interrupt gate
	dw (isr13-$$+0x0900-2) & 0xFFFF	;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 10001110b	;flags byte
	dw (isr13-$$+0x0900) >> 16	;the upper byte of the offset
idt14:	;interrupt gate
	dw (isr14-$$+0x0900-2) & 0xFFFF	;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 10001110b	;flags byte
	dw (isr14-$$+0x0900) >> 16	;the upper byte of the offset
idt15:	;interrupt gate
	dw (isr9-$$+0x0900-2) & 0xFFFF	;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 00001110b	;flags byte (not present, reserved)
	dw (isr9-$$+0x0900) >> 16	;the upper byte of the offset
idt16:	;interrupt gate
	dw (isr16-$$+0x0900-2) & 0xFFFF	;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 10001110b	;flags byte
	dw (isr16-$$+0x0900) >> 16	;the upper byte of the offset
idt17:	;interrupt gate
	dw (isr17-$$+0x0900-2) & 0xFFFF	;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 10001110b	;flags byte
	dw (isr17-$$+0x0900) >> 16	;the upper byte of the offset
idt18:	;interrupt gate
	dw (isr18-$$+0x0900-2) & 0xFFFF	;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 10001110b	;flags byte
	dw (isr18-$$+0x0900) >> 16	;the upper byte of the offset
idt19:	;interrupt gate
	dw (isr19-$$+0x0900-2) & 0xFFFF	;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 10001110b	;flags byte
	dw (isr19-$$+0x0900) >> 16	;the upper byte of the offset
;20 - 31 are reserved, 32-255 are usable for anything
idt_end:
idt_desc:				;IDT descriptor
	dw idt_end - idt - 1	;limit
	dd idt		;address for idt

Zero db 'Divide by zero error!', 10, 0
isr0:
;fault, no error code
;display message, stop, because returning brings it back
	mov ax, 10h
	mov ds, ax			;data segment (we can use variables with their name now)
	push Zero
	call __Z7displayPc		;print string (C++ function)
	cli				;disable interrupts
	hlt				;hang
One db 'Debug exception', 10, 0
isr1:
;trap or fault, no error code
;examine DR6 and other debug registers to determine type (not necessary though)
	mov ax, 10h
	mov ds, ax			;data segment (we can use variables with their name now)
	push One
	call __Z7displayPc		;print string (C++ function)
	cli				;disable interrupts
	hlt				;hang
Two db 'NMI Interrupt', 10, 0
isr2:
;interrupt, no error code
;hmm what to do? i dont know...
	mov ax, 10h
	mov ds, ax			;data segment (we can use variables with their name now)
	push Two
	call __Z7displayPc		;print string (C++ function)
	cli				;disable interrupts
	hlt				;hang
Three db 'Breakpoint', 10, 0
isr3:
;trap, no error code
	mov ax, 10h
	mov ds, ax			;data segment (we can use variables with their name now)
	push Three
	call __Z7displayPc		;print string (C++ function)
	cli				;disable interrupts
	hlt				;hang
Four db 'Overflow', 10, 0
isr4:
;trap, no error code
mov ax, 10h
	mov ds, ax			;data segment (we can use variables with their name now)
	push Four
	call __Z7displayPc		;print string (C++ function)
	cli				;disable interrupts
	hlt				;hang
Five db 'Bounds range exceeded', 10, 0
isr5:
;fault, no error code
mov ax, 10h
	mov ds, ax			;data segment (we can use variables with their name now)
	push Five
	call __Z7displayPc		;print string (C++ function)
	cli				;disable interrupts
	hlt				;hang
Six db 'Invalid opcode', 10, 0
isr6:
;fault, no error code
mov ax, 10h
	mov ds, ax			;data segment (we can use variables with their name now)
	push Six
	call __Z7displayPc		;print string (C++ function)
;our stack contains: EIP, CS
	pop ax
	mov es, ax
	pop ebx
	cmp WORD [ES:EBX], 0x090F	;this should be the opcode for wbinvd
	je .Wbinvd
	jmp .Other
.Wbinvd
	add ebx, 2		;skip that instruction (it wont hurt anything)
	push ebx
	push ax		;add stuff to the stack for a proper iret
	iret
.Other
	cli				;disable interrupts
	hlt				;hang
Seven db 'Device not available', 10, 0
isr7:
;fault, no error code
;this is confusing
mov ax, 10h
	mov ds, ax			;data segment (we can use variables with their name now)
	push Seven
	call __Z7displayPc		;print string (C++ function)
	cli				;disable interrupts
	hlt				;hang
Eight db 'Double - fault', 10, 0
isr8:
;abort, error code does exist (it is zero)
;there is no return from here, the program must be closed, and data logged
mov ax, 10h
	mov ds, ax			;data segment (we can use variables with their name now)
	push Eight
	call __Z7displayPc		;print string (C++ function)
	pop eax
	cli				;disable interrupts
	hlt				;hang
Nine db 'Coprocessor segment overrun', 10, 0
isr9:
;abort, no error code
;FPU must be restarted (so we won't return for now)
mov ax, 10h
	mov ds, ax			;data segment (we can use variables with their name now)
	push Nine
	call __Z7displayPc		;print string (C++ function)
	cli				;disable interrupts
	hlt				;hang
Ten db 'Invalid TSS exception', 10, 0
isr10:
;fault, error code present
;must use a task gate, to preserve stability
mov ax, 10h
	mov ds, ax			;data segment (we can use variables with their name now)
	push Ten
	call __Z7displayPc		;print string (C++ function)
	pop eax
	cli				;disable interrupts
	hlt				;hang
Eleven db 'Segment not present', 10, 0
isr11:
;fault, error code present
	mov ax, 10h
	mov ds, ax			;data segment (we can use variables with their name now)
	push Eleven
	call __Z7displayPc		;print string (C++ function)
	pop eax
	cli				;disable interrupts
	hlt				;hang
Twelve db 'Stack fault exception', 10, 0
isr12:
;fault, error code present
	mov ax, 10h
	mov ds, ax			;data segment (we can use variables with their name now)
	push Twelve
	call __Z7displayPc		;print string (C++ function)
	pop eax
	cli				;disable interrupts
	hlt				;hang
Thirteen db 'General Protection Fault', 10, 0
isr13:
;fault, error code present
	mov ax, 10h
	mov ds, ax			;data segment (we can use variables with their name now)
	push Thirteen
	call __Z7displayPc		;print string (C++ function)
	pop eax
	cli				;disable interrupts
	hlt				;hang
Fourteen db 'Page Fault Exception', 10, 0
isr14:
;fault, special error code format same size though
;call through task gate, to allow page faulting during task switches
	mov ax, 10h
	mov ds, ax			;data segment (we can use variables with their name now)
	push Fourteen
	call __Z7displayPc		;print string (C++ function)
	pop eax
	cli				;disable interrupts
	hlt				;hang
Sixteen db 'x87 FPU Floating-Point Error', 10, 0
isr16:
;fault, no error code
	mov ax, 10h
	mov ds, ax			;data segment (we can use variables with their name now)
	push Sixteen
	call __Z7displayPc		;print string (C++ function)
	cli				;disable interrupts
	hlt				;hang
Seventeen db 'Alignment Check Exception', 10, 0
isr17:
;fault, error code present
	mov ax, 10h
	mov ds, ax			;data segment (we can use variables with their name now)
	push Seventeen
	call __Z7displayPc		;print string (C++ function)
	pop eax
	cli				;disable interrupts
	hlt				;hang
Eighteen db 'Machine-Check Exception', 10, 0
isr18:
;abort, no error code
	mov ax, 10h
	mov ds, ax			;data segment (we can use variables with their name now)
	push Eighteen
	call __Z7displayPc		;print string (C++ function)
	cli				;disable interrupts
	hlt				;hang
Nineteen db 'SIMD Floating-Point Exception', 10, 0
isr19:
;fault, no error code
	mov ax, 10h
	mov ds, ax			;data segment (we can use variables with their name now)
	push Nineteen
	call __Z7displayPc		;print string (C++ function)
	cli				;disable interrupts
	hlt				;hang
