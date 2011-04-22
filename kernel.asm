[BITS 32]
[global start] 
[extern _main]			;this is in our C++ code (it is the function that starts it all off)
[extern __main]			;this is in our C support code
[extern __atexit] 		;this is in our C support code
[extern __Z7displayPc]		;void display(char *chr)
[global __Z12EnablePagingv]	;void EnablePaging(void)
[extern __Z11PrintNumberm]	;void PrintNumber(unsigned long)
[global __Z11ReadSectorsmmmh]	;bool ReadSectors(unsigned long SectorNumber,unsigned long NumSectors,unsigned char DriveNum)
[global __Z10ReadSectormmh]	;bool ReadSector(unsigned long SectorNumber, unsigned char DriveNum)
[global __Z12EnableFloppyv]	;bool EnableFloppyFunctions()
	mov ax, 0x10
	mov ds, ax			;set the segment registers
	mov ss, eax			;and stack
	xor eax, eax
	mov es, eax
	mov fs, eax
	mov gs, eax
	lgdt [ds:gdt_desc]
		;this is so the place where the GDT used to be wont be a problem when the stack overwrites it
	mov eax, 0x0900
	mov esp, eax
	lidt [idt_desc]	;load the IDT
	sti			;enable interrupts
	mov al, 0		;enable IRQ's
	out 0x21, al	;enable IRQ's
	call enableA20
	start:
	call __main
	call _main 		;call int main(void), which is located in our C++ code
	cmp eax, 0
	je Yay
	push Bad
	call __Z7displayPc		;print string (C++ function)
	jmp Yay2
Yay:
	push Good
	call __Z7displayPc		;print string (C++ function)
	Yay2:
	call __atexit
	jmp $
gdt:                    ; Address for the GDT
gdt_null:               ; Null Segment
	dd 0
	dd 0
gdt_code:               ; Code segment, read/execute, nonconforming
	dw 0xFFFF
	dw 0x0000
	db 0
	db 10011010b	;non-system descriptor (bit 4)
	db 11001111b
	db 0
gdt_data:               ; Data segment, read/write
	dw 0xFFFF
	dw 0x0000
	db 0
	db 10010010b	;non system descriptor (bit 4)
	db 11001111b
	db 0
gdt_end:				; Used to calculate the size of the GDT
gdt_desc:				; The GDT descriptor
	dw gdt_end - gdt - 1	; Limit (size)
	dd gdt			; Address of the GDT

Bad db "Doors has exited with an error.", 0
Good db "Doors has shutdown properly (It is now safe to turn off your computer).", 0

;interrupt descriptor format
;interrupt gate
	;low word of handler offset
	;word segment selector
	;zero_byte				;low nibble is reserved
	;flag byte p dpl 0 1 1 1 0	;32 bits for size of gate
	;high word of handler offset

enableA20:
	pusha
	cli                                    ; Disable all irqs
	cld
	mov   al,255                           ; Mask all irqs
	out   0xa1,al
	out   0x21,al
l.5:	in    al,0x64                          ; Enable A20
	test  al,2                             ; Test the buffer full flag
	jnz   l.5                              ; Loop until buffer is empty
	mov   al,0xD1                          ; Keyboard: write to output port
	out   0x64,al                          ; Output command to keyboard
l.6:	in    al,0x64
	test  al,2
	jnz   l.6                              ; Wait 'till buffer is empty again
	mov   al,0xDF                          ; keyboard: set A20
	out   0x60,al                          ; Send it to the keyboard controller
	mov   cx,14h
l.7:                                           ; this is approx. a 25uS delay to wait
	out   0edh,ax                          ; for the kb controler to execute our
	loop  l.7                              ; command.
	sti
	popa
	ret
	call A20Check
	jnz .Keepgoing
	jmp $
.Keepgoing
	ret

A20Check:
	push ax
	push ds
	push es
	xor ax,ax
	mov ds,ax
	dec ax
	mov es,ax
	mov ax,[es:10h]		; read word at FFFF:0010 (1 meg)
	not ax			; 1's complement
	push word [0]		; save word at 0000:0000 (0)
	mov [0],ax	; word at 0 = ~(word at 1 meg)
	mov ax,[0]	; read it back
	cmp ax,[es:10h]	; fail if word at 0 == word at 1 meg
	pop word [0]
	pop es
	pop ds
	pop ax
	ret		; if ZF=1, the A20 gate is NOT enabled

idt:				;address for the IDT
;10, 14 - use task gate
idt0:	;interrupt gate
      dw (isr0+0x0900-$$) & 0xFFFF  ;get the lower word for the offset
	dw 0x08				;the code segment for our segment
	db 0
	db 10001110b
	dw (isr0+0x0900-$$) >> 16	;the upper byte of the offset
idt1:	;interrupt gate
	dw (isr1-$$+0x0900) & 0xFFFF	;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 10001110b	;flags byte
	dw (isr1-$$+0x0900) >> 16	;the upper byte of the offset
idt2:	;interrupt gate @@@
	dw (isr2-$$+0x0900) & 0xFFFF	;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 00001110b	;flags byte (not present, supposedly reserved)
	dw (isr2-$$+0x0900) >> 16	;the upper byte of the offset
idt3:	;interrupt gate
	dw (isr3-$$+0x0900) & 0xFFFF	;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 10001110b	;flags byte
	dw (isr3-$$+0x0900) >> 16	;the upper byte of the offset
idt4:	;interrupt gate
	dw (isr4-$$+0x0900) & 0xFFFF	;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 10001110b	;flags byte
	dw (isr4-$$+0x0900) >> 16	;the upper byte of the offset
idt5:	;interrupt gate
	dw (isr5-$$+0x0900) & 0xFFFF	;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 10001110b	;flags byte
	dw (isr5-$$+0x0900) >> 16	;the upper byte of the offset
idt6:	;interrupt gate
	dw (isr6-$$+0x0900) & 0xFFFF	;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 10001110b	;flags byte
	dw (isr6-$$+0x0900) >> 16	;the upper byte of the offset
idt7:	;interrupt gate
	dw (isr7-$$+0x0900) & 0xFFFF	;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 10001110b	;flags byte
	dw (isr7-$$+0x0900) >> 16	;the upper byte of the offset
idt8:	;interrupt gate
	dw (isr8-$$+0x0900) & 0xFFFF	;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 10001110b	;flags byte
	dw (isr8-$$+0x0900) >> 16	;the upper byte of the offset
idt9:	;interrupt gate
	dw (isr9-$$+0x0900) & 0xFFFF	;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 00001110b	;flags byte
	dw (isr9-$$+0x0900) >> 16	;the upper byte of the offset
idt10:	;interrupt gate
	dw (isr10-$$+0x0900) & 0xFFFF	;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 10001110b	;flags byte
	dw (isr10-$$+0x0900) >> 16	;the upper byte of the offset
idt11:	;interrupt gate
	dw (isr11-$$+0x0900) & 0xFFFF	;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 10001110b	;flags byte
	dw (isr11-$$+0x0900) >> 16	;the upper byte of the offset
idt12:	;interrupt gate
	dw (isr12-$$+0x0900) & 0xFFFF	;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 10001110b	;flags byte
	dw (isr12-$$+0x0900) >> 16	;the upper byte of the offset
idt13:	;interrupt gate
	dw (isr13-$$+0x0900) & 0xFFFF	;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 10001110b	;flags byte
	dw (isr13-$$+0x0900) >> 16	;the upper byte of the offset
idt14:	;interrupt gate
	dw (isr14-$$+0x0900) & 0xFFFF	;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 10001110b	;flags byte
	dw (isr14-$$+0x0900) >> 16	;the upper byte of the offset
idt15:	;interrupt gate @@@
	dw (isr10-$$+0x0900) & 0xFFFF	;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 00001110b	;flags byte (not present, reserved)
	dw (isr10-$$+0x0900) >> 16	;the upper byte of the offset
idt16:	;interrupt gate
	dw (isr16-$$+0x0900) & 0xFFFF	;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 10001110b	;flags byte
	dw (isr16-$$+0x0900) >> 16	;the upper byte of the offset
idt17:	;interrupt gate
	dw (isr17-$$+0x0900) & 0xFFFF	;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 10001110b	;flags byte
	dw (isr17-$$+0x0900) >> 16	;the upper byte of the offset
idt18:	;interrupt gate
	dw (isr18-$$+0x0900) & 0xFFFF	;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 10001110b	;flags byte
	dw (isr18-$$+0x0900) >> 16	;the upper byte of the offset
idt19:	;interrupt gate
	dw (isr19-$$+0x0900) & 0xFFFF	;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 10001110b	;flags byte
	dw (isr19-$$+0x0900) >> 16	;the upper byte of the offset
times 12 dw 0, 0x08, 0000111000000000b, 0
	;for all of those reserved interrupts
idt32:	;MASTER IRQ 0
	dw 0			;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 10001110b	;flags byte
	dw 0			;the upper byte of the offset
idt33:	;MASTER IRQ 1
	dw 0			;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 10001110b	;flags byte
	dw 0			;the upper byte of the offset
idt34:	;MASTER IRQ 2
	dw 0			;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 10001110b	;flags byte
	dw 0			;the upper byte of the offset
idt35:	;MASTER IRQ 3
	dw 0			;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 10001110b	;flags byte
	dw 0			;the upper byte of the offset
idt36:	;MASTER IRQ 4
	dw 0			;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 10001110b	;flags byte
	dw 0			;the upper byte of the offset
idt37:	;MASTER IRQ 5
	dw 0			;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 10001110b	;flags byte
	dw 0			;the upper byte of the offset
idt38:	;MASTER IRQ 6
	dw 0			;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 10001110b	;flags byte
	dw 0			;the upper byte of the offset
idt39:	;MASTER IRQ 7
	dw 0			;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 10001110b	;flags byte
	dw 0			;the upper byte of the offset

;20 - 31 are reserved, 32-39 used for master IRQ 0 - 7, 40 - 47 slave IRQ 0 - 7, 
	;48-255 are usable for anything
idt_end:
idt_desc:				;IDT descriptor
	dw idt_end - idt - 1	;limit
	dd idt		;address for idt
Code dd 0	;this stores any error code that needs to be examined in the following routines
Zero db 'Divide by zero error!', 10, 0
isr0:
;fault, no error code
;display message, stop, because returning brings it back
	mov ax, 10h
	mov ds, ax			;data segment (we can use variables with their name now)
	push Zero
	call __Z7displayPc		;print string (C++ function)
	jmp $
One db 'Debug exception', 10, 0
isr1:
;trap or fault, no error code
;examine DR6 and other debug registers to determine type (not necessary though)
	mov ax, 10h
	mov ds, ax			;data segment (we can use variables with their name now)
	push One
	call __Z7displayPc		;print string (C++ function)
	jmp $
Two db 'NMI Interrupt', 10, 0
isr2:
;interrupt, no error code
;hmm what to do? i dont know...
	mov ax, 10h
	mov ds, ax			;data segment (we can use variables with their name now)
	push Two
	call __Z7displayPc		;print string (C++ function)
	jmp $
Three db 'Breakpoint', 10, 0
isr3:
;trap, no error code
	mov ax, 10h
	mov ds, ax			;data segment (we can use variables with their name now)
	push Three
	call __Z7displayPc		;print string (C++ function)
	jmp $
Four db 'Overflow', 10, 0
isr4:
;trap, no error code
	mov ax, 10h
	mov ds, ax			;data segment (we can use variables with their name now)
	push Four
	call __Z7displayPc		;print string (C++ function)
	jmp $
Five db 'Bounds range exceeded', 10, 0
isr5:
;fault, no error code
	mov ax, 10h
	mov ds, ax			;data segment (we can use variables with their name now)
	push Five
	call __Z7displayPc		;print string (C++ function)
	jmp $
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
	jmp $
Seven db 'Device not available', 10, 0
isr7:
;fault, no error code
;this is confusing
	mov ax, 10h
	mov ds, ax			;data segment (we can use variables with their name now)
	push Seven
	call __Z7displayPc		;print string (C++ function)
	jmp $
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
	jmp $
Ten db 'Invalid TSS exception', 10, 0
isr10:
;fault, error code present
;must use a task gate, to preserve stability
	mov ax, 10h
	mov ds, ax			;data segment (we can use variables with their name now)
	push Ten
	call __Z7displayPc		;print string (C++ function)
	pop eax
	jmp $
Eleven db 'Segment not present', 10, 0
isr11:
;fault, error code present
	mov ax, 10h
	mov ds, ax			;data segment (we can use variables with their name now)
	push Eleven
	call __Z7displayPc	;print string (C++ function)
	pop eax
	jmp $
Twelve db 'Stack fault exception', 10, 0
isr12:
;fault, error code present
	mov ax, 10h
	mov ds, ax			;data segment (we can use variables with their name now)
	push Twelve
	call __Z7displayPc	;print string (C++ function)
	pop eax
	jmp $
Thirteen db 'General Protection Fault', 10, 0
isr13:
;fault, error code present
	mov ax, 10h
	mov ds, ax			;data segment (we can use variables with their name now)
	push Thirteen
	call __Z7displayPc	;print string (C++ function)
	pop eax
	jmp $
Fourteen db 'Page Fault Exception', 10, 0
_Fourteen db 'A reserved bit has been set in the page directory!', 0
_2Fourteen db 'A page level protection violation has occurred!', 0
_3Fourteen db 'A page that does not exist in RAM has been accessed', 0
Location dd 0		;hold the location of the page fault location
_EAX dd 0
isr14:
;fault, special error code format same size though
;call through task gate, to allow page faulting during task switches
;first, place all regs 
	push eax
	mov ax, 0x10
	mov ds, ax			;data segment (we can use variables with their name now)
	pop eax
	;store eax in a variable, so we can allow for an error code
	mov [_EAX], eax
	;get the error code
	pop eax
	mov [Code], eax
	mov eax, _EAX
	pushad			;do a popad	before returning to code
	mov eax, cr2
	push eax			;save that address to the stack
	push Fourteen
	call __Z7displayPc		;print string (C++ function)
	;check that a reserved bit was not set in the page directory (thats bad)
	mov eax, Code
	and eax, 1000b
	cmp eax, 1000b
	jne .Yay
	push _Fourteen
	call __Z7displayPc
	;display cr2
	jmp $
.Yay
	;CS|EIP|PUSHAD|CR2|
	mov eax, Code
	and eax, 1
	cmp eax, 1
	jne Ok
	;page level protection violation
	push _2Fourteen
	call __Z7displayPc
	jmp $
Ok:
	push _3Fourteen
	call __Z7displayPc
	jmp $
Sixteen db 'x87 FPU Floating-Point Error', 10, 0
isr16:
;fault, no error code
	mov ax, 10h
	mov ds, ax			;data segment (we can use variables with their name now)
	push Sixteen
	call __Z7displayPc		;print string (C++ function)
	jmp $
Seventeen db 'Alignment Check Exception', 10, 0
isr17:
;fault, error code present
	mov ax, 10h
	mov ds, ax			;data segment (we can use variables with their name now)
	push Seventeen
	call __Z7displayPc		;print string (C++ function)
	pop eax
	jmp $
Eighteen db 'Machine-Check Exception', 10, 0
isr18:
;abort, no error code
	mov ax, 10h
	mov ds, ax			;data segment (we can use variables with their name now)
	push Eighteen
	call __Z7displayPc		;print string (C++ function)
	jmp $
Nineteen db 'SIMD Floating-Point Exception', 10, 0
isr19:
;fault, no error code
	mov ax, 10h
	mov ds, ax			;data segment (we can use variables with their name now)
	push Nineteen
	call __Z7displayPc		;print string (C++ function)
	jmp $
Mode db 0	;defines what the interrupt should be expecting to do when called
IRQM6 db 'FDC has fired an interrupt!', 10, 13, 0
irqM6:
;this is IRQ 6 from the master PIC
	push IRQM6
	call __Z7displayPc
	;manual EOI before the interrupt has ended
	mov al, 0x20
	out 0x20, al
	;now return to wherever execution was before this interrupt
	iret

__Z12EnablePagingv:
	mov eax, 0x0100000	;set the base of the page directory (1 MB)
	mov cr3, eax		;time to set the paging bit
	mov eax, cr0
	or eax, 1110000000000000000000000000000b
	mov cr0, eax
	ret

__Z12EnableFloppyv:
	mov al, 0x0
	mov dx, 0x3F2
	out dx, al		;reset the floppy drive controller and turn off all motors
	ret;

Yes db 'Thats right', 0
__Z10ReadSectormmh:		;reads one sector from a floppy disk (0x0F03)
	push ebp
	mov ebp, esp
	;sub esp, 0x1		;1 byte for local variables
	;activate the appropiate floppy drive motor
	mov eax, [ebp + 8]	;the farthest left parameter
	leave
	ret

__Z11ReadSectorsmmmh:		;reads sectors from a floppy disk by making calls to ReadSector
	mov eax, 0
	ret