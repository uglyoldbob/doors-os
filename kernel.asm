[BITS 32]
cpu 386
[global _starting]
[extern main]			;this is in our C++ code (it is the function that starts it all off)
[extern _main]			;this is in our C support code
[extern _atexit] 		;this is in our C support code
[extern display]		;void display(char *chr)
[global EnablePaging]	;void EnablePaging(void)
[extern PrintNumber]	;void PrintNumber(unsigned long)
[global ReadSectors]	;bool ReadSectors(unsigned long SectorNumber,unsigned long NumSectors,unsigned char DriveNum)
[global ReadSector]	;bool ReadSector(unsigned long SectorNumber, unsigned char DriveNum)
[global EnableFloppy]	;bool EnableFloppyFunctions()
[global EnableKeyboard]	;unsigned long int EnableKeyboard(void)
[global Milli]		;unsigned long Milli()
[extern LoopsPerSecond]	;unsigned long LoopsPerSecond
[extern delay]		;void delay(unsigned long a)
[global getEIP]		;returns eip

	_starting:
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
	;mov al, 0x0
	;mov dx, 0x3F2
	;out dx, al		;reset the floppy drive controller and turn off all motors
	call enableA20
	;re-enable interrupts
	mov al, 0	;enable IRQ's
	out 0x21, al	;enable IRQ's
	;setup the clock (irq 0) to have a frequency of about 1000 Hz
	;1193180 / Hz is the number to send through to port 0x40
	mov al, 0x34
	out 0x43, al
	mov al, 0xA9	;lower byte
	out 0x40, al
	mov al, 0x04	;upper byte
	out 0x40, al
	;call BogoMips
	;cmp eax, 1
	;je Evil 
;	mov bh, 30
;	mov ah, 0x9B
;	mov bl, 0x2E
;	call MakeNoise
	call _main
	call main 		;call int main(void), which is located in our C++ code
	cmp eax, 0
	jmp Yay

	je Yay
Evil:
	push Bad
	call display		;print string (C++ function)
	pop eax
	jmp Yay2
Yay:
	push Good
	call display		;print string (C++ function)
	pop eax
	Yay2:
	call _atexit
	jmp $

getEIP:
	mov eax, [esp]
	ret

WaitForKeyboard:
	push ax
.1
	in al, 0x64
	and al, 0x02
	jnz .1
	pop ax
	ret

MakeNoise:
	;low byte	-bl
	;high byte	-ah
	;number of clock cycles to last - bh
	;time to test some new code
	;0x1234DD / 400 = 2E9B
	mov al, 0xB6
	out 0x43, al	;low byte
	mov al, bl
	out 0x42, al	;high byte
	mov al, ah
	out 0x42, al
	in al, 0x61
	or al, 3
	out 0x61, al
	mov ah, 0
	mov bx, ax
	call Delay
	in al, 0x61
	and al, 0xFC
	out 0x61, al
	ret

Delay:	;load bx with number of clock cycles to delay
	;need to make sure that (timer + bx > timer) if not reset timer
	push eax
	mov ax, [timer]
	add ax, bx	;timer + delay
	cmp ax, [timer]
	jg .safe
	mov ax, 0
	mov word [timer], 0
	add ax, bx	;timer (new) + delay
.safe
	cmp ax, [timer]
	jge .safe
	pop eax		;almost forgot this (that would be bad)
	ret	

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
	mov   al, 255                           ; Mask all irqs
	out   0xA1, al
	out   0x21, al
l.5:	in    al, 0x64                          ; Enable A20
	test  al, 2                             ; Test the buffer full flag
	jnz   l.5                              ; Loop until buffer is empty
	mov   al, 0xD1                          ; Keyboard: write to output port
	out   0x64, al                          ; Output command to keyboard
l.6:	in    al, 0x64
	test  al, 2
	jnz   l.6                              ; Wait 'till buffer is empty again
	mov   al, 0xDF                          ; keyboard: set A20
	out   0x60, al                          ; Send it to the keyboard controller
	mov   cx, 0x14
l.7:                                           ; this is approx. a 25uS delay to wait
	out   0xED, ax                          ; for the kb controler to execute our
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
	dw (isr15-$$+0x0900) & 0xFFFF	;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 00001110b	;flags byte (not present, reserved)
	dw (isr15-$$+0x0900) >> 16	;the upper byte of the offset
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
	dw (irqM0-$$+0x0900) & 0xFFF			;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 10001110b	;flags byte
	dw (irqM0-$$+0x0900) >> 16			;the upper byte of the offset
idt33:	;MASTER IRQ 1
	dw (irqM1-$$+0x0900) & 0xFFFF			;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 10001110b	;flags byte
	dw (irqM1-$$+0x0900) >> 16			;the upper byte of the offset
idt34:	;MASTER IRQ 2
	dw (irqM2-$$+0x0900) & 0xFFFF			;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 10001110b	;flags byte
	dw (irqM2-$$+0x0900) >> 16			;the upper byte of the offset
idt35:	;MASTER IRQ 3
	dw (irqM3-$$+0x0900) & 0xFFFF			;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 10001110b	;flags byte
	dw (irqM3-$$+0x0900) >> 16			;the upper byte of the offset
idt36:	;MASTER IRQ 4
	dw (irqM4-$$+0x0900) & 0xFFFF			;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 10001110b	;flags byte
	dw (irqM4-$$+0x0900) >> 16			;the upper byte of the offset
idt37:	;MASTER IRQ 5
	dw (irqM5-$$+0x0900) & 0xFFFF			;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 10001110b	;flags byte
	dw (irqM5-$$+0x0900) >> 16			;the upper byte of the offset
idt38:	;MASTER IRQ 6
	dw (irqM6-$$+0x0900) & 0xFFFF	;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 10001110b	;flags byte
	dw (irqM6-$$+0x0900) >> 16			;the upper byte of the offset
idt39:	;MASTER IRQ 7
	dw (irqM7-$$+0x0900) & 0xFFFF			;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 10001110b	;flags byte
	dw (irqM7-$$+0x0900) >> 16			;the upper byte of the offset

;20 - 31 are reserved, 32-39 used for master IRQ 0 - 7, 40 - 47 slave IRQ 0 - 7, 
	;48-255 are usable for anything
idt_end:
idt_desc:				;IDT descriptor
	dw idt_end - idt - 1	;limit
	dd idt		;address for idt

Milli:
	mov eax, [timer]
	ret

timer dd 0	;this is the 16 bit timer used for timing in the OS
		;60 Hz is the frequency (hopefully)

irqM0:
	push ax
	inc dword [timer]
	;manual EOI before the interrupt has ended
	mov al, 0x20
	out 0x20, al
	pop ax
	iret

ScanSet db 0, '1234567890-=', 8, 9, 'qwertyuiop[]', 0, 0, 'asdfghjkl;', 0x27, '`', 0, '\zxcvbnm,./', 0, '*', 0, ' ', 0, 0, 0, 0, 0, 0 ,0 ,0 ,0, 0, 0, 0, 0, '789-456+1230', 0
ShiftSet db 0,'!@#$%^&*()_+', 8, 9, 'QWERTYUIOP{}', 0, 0, 'ASDFGHJKL:"~', 0, '|ZXCVBNM<>?', 0, '*', 0, ' ', 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, '789-456+1230', 0
Temp db '?', 0	;this is used to store what will be displayed for the key that was just pressed (0 means nothing will be displayed)
Newline db 13, 10, 0	;this is needed because when you press enter, two bytes need to be sent for it to work properly
Buffer db 0,0,0,0,0,0	;used to process multi byte scancodes
Shift db 0		;determines if a shift is currently being held down
Alt db 0		;determines if an alt key is being held down
Ctrl db 0		;same thing for ctrl key
OneByte db 'One byte code*', 0
TwoByte db 'Two byte code*', 0
SixByte db 'Six byte code*', 0
FourByte db 'Four byte code*', 0

;0xE0 0x2A 0xE0, 0x53
irqM1:
	pusha
	xor eax, eax
	in al, 0x60
	;have we already recieved the first byte of a multi-byte scancode, if so go to the handler for multibyte scancodes
	mov bl, [Buffer]
	cmp bl, 0
	jnz irqM1_Multi
	;is this the first byte of a multi-byte scancode?
	;0xE0 and 0xE1 are the only two bytes that are not single byte scancodes
	cmp al, 0xE0
	jne irqM1_single
	mov [Buffer], al
	jmp irqM1_end
irqM1_single:
	cmp al, 0xE1
	jne irqM1_reallySingle
	mov [Buffer], al
	jmp irqM1_end
irqM1_reallySingle:
;@handle 1byte scancodes
	push OneByte
	call display
	pop ebx
	jmp irqM1_end
irqM1_Multi:	;handles multi-byte scancodes (2,4,6)
	;we know there is one byte in the scancode buffer
	mov bl, [Buffer + 1]
	cmp bl, 0
	jne irqM1_Multi2	;we do not have the second byte but we might have the 3,4,5, or 6 byte
	mov [Buffer + 1], al
	;is the second byte of a longer scancode? (if so, processing is done with this byte)
	;0x2A, 0xB7, 0x1D (check first byte of buffer)
	cmp al, 0x2A
	je irqM1_end
	cmp al, 0xB7
	je irqM1_end
	mov bl, [Buffer]
	cmp bl, 0xE1
	je irqM1_end
;@handle 2 byte scancodes
	push TwoByte
	call display
	pop ebx
	;done handling 2 byte scancodes
	xor bx, bx
	mov [Buffer], bx	;this will clear two bytes of the buffer
	jmp irqM1_end
irqM1_Multi2:	;we know that there are two bytes in the scancode buffer
	mov bl, [Buffer + 2]
	cmp bl, 0
	jnz irqM1_Multi3
	mov [Buffer + 2], al
	;there are no three byte scancodes that i know of
	jmp irqM1_end
irqM1_Multi3:	;there are three bytes in the scancode buffer
	mov bl, [Buffer + 3]
	cmp bl, 0
	jnz irqM1_Multi4
	mov [Buffer + 3], al
	;check for scancodes that are longer and do not need to be processed yet
	mov bl, [Buffer]
	cmp bl, 0xE1	;afaik, pause is the only 6 byte scancode
	je irqM1_end
;@handle 4 byte scancodes
	push FourByte
	call display
	pop ebx
	;done handling scancode
	xor ebx, ebx
	mov [Buffer], ebx	;this will clear 4 bytes of the buffer
	jmp irqM1_end
irqM1_Multi4:	;there are four bytes in the scancode buffer
	mov bl, [Buffer + 4]
	cmp bl, 0
	jnz irqM1_Multi5
	mov [Buffer + 4], al
	;there are no 5 byte scancodes that i know of
	jmp irqM1_end
irqM1_Multi5:	;there are five bytes in the scancode buffer
;@handle 6 byte scancodes
	mov [Buffer + 5], al
	push SixByte
	call display
	pop ebx
	;done handling 6 byte scancode(s)
	xor ebx, ebx
	mov [Buffer], ebx
	mov [Buffer + 4], bl	;this should clear 6 bytes the easy way
	nop
	jmp irqM1_end
	
irqM1_end:	;when all handling for the current scancode byte is complete
	mov al, 0x20
	out 0x20, al
	popa
	iret
	
IRQM2 db 'IRQ2', 13, 10, 0
irqM2:
	push eax
	;manual EOI before the interrupt has ended
	push IRQM2
	call display
	pop eax
	mov al, 0x20
	out 0x20, al
	pop eax
	iret

IRQM3 db 'IRQ3', 13, 10, 0
irqM3:
	push eax
	;manual EOI before the interrupt has ended
	push IRQM3
	call display
	pop eax
	mov al, 0x20
	out 0x20, al
	pop eax
	iret

IRQM4 db 'IRQ4', 13, 10, 0
irqM4:
	push eax
	;manual EOI before the interrupt has ended
	push IRQM4
	call display
	pop eax
	mov al, 0x20
	out 0x20, al
	pop eax
	iret

IRQM5 db 'IRQ5', 13, 10, 0
irqM5:
	push eax
	;manual EOI before the interrupt has ended
	push IRQM5
	call display
	pop eax
	mov al, 0x20
	out 0x20, al
	pop eax
	iret

BytesDone dd 0	;defines what the interrupt should be expecting to do when called
IRQM6 db 'FDC has fired an interrupt!', 10, 13, 0
irqM6:
;this is IRQ 6 from the master PIC
	pusha
	;determine what this means
	inc dword [BytesDone]
;	push IRQM6
;	call display
;	pop eax
	;now return to wherever execution was before this interrupt
	popa
	iret

IRQM7 db 'IRQ7', 13, 10, 0
irqM7:
	push eax
	;manual EOI before the interrupt has ended
	push IRQM7
	call display
	pop eax
	mov al, 0x20
	out 0x20, al
	pop eax
	iret

Code dd 0	;this stores any error code that needs to be examined in the following routines
Zero db 'Divide by zero error!', 10, 0
isr0:
;fault, no error code
;display message, stop, because returning brings it back
	mov ax, 10h
	mov ds, ax			;data segment (we can use variables with their name now)
	push Zero
	call display		;print string (C++ function)
	pop eax
	jmp $
One db 'Debug exception', 10, 0
isr1:
;trap or fault, no error code
;examine DR6 and other debug registers to determine type (not necessary though)
	mov ax, 10h
	mov ds, ax			;data segment (we can use variables with their name now)
	push One
	call display		;print string (C++ function)
	pop eax
	jmp $
Two db 'NMI Interrupt', 10, 0
isr2:
;interrupt, no error code
;hmm what to do? i dont know...
	mov ax, 10h
	mov ds, ax			;data segment (we can use variables with their name now)
	push Two
	call display		;print string (C++ function)
	pop eax
	jmp $
Three db 'Breakpoint', 10, 0
isr3:
;trap, no error code
	pusha
	mov ax, 10h
	mov ds, ax			;data segment (we can use variables with their name now)
	mov eax, [ebp + 44]
	push eax
	call PrintNumber		;print string (C++ function)
	pop eax
	push Newline
	call display
	pop eax
	mov eax, [ebp + 40]
	push eax
	call PrintNumber
	pop eax
	push Newline
	call display
	pop eax
	popa
	iret
Four db 'Overflow', 10, 0
isr4:
;trap, no error code
	mov ax, 10h
	mov ds, ax			;data segment (we can use variables with their name now)
	push Four
	call display		;print string (C++ function)
	pop eax
	jmp $
Five db 'Bounds range exceeded', 10, 0
isr5:
;fault, no error code
	mov ax, 10h
	mov ds, ax			;data segment (we can use variables with their name now)
	push Five
	call display		;print string (C++ function)
	pop eax
	jmp $
Six db 'Invalid opcode', 10, 0
isr6:
;fault, no error code
	mov ax, 10h
	mov ds, ax			;data segment (we can use variables with their name now)
	push Six
	call display		;print string (C++ function)
	pop eax
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
	call display		;print string (C++ function)
	pop eax
	jmp $
Eight db 'Double - fault', 10, 0
isr8:
;abort, error code does exist (it is zero)
;there is no return from here, the program must be closed, and data logged
	mov ax, 10h
	mov ds, ax			;data segment (we can use variables with their name now)
	push Eight
	call display		;print string (C++ function)
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
	call display		;print string (C++ function)
	pop eax
	jmp $
Ten db 'Invalid TSS exception', 10, 0
isr10:
;fault, error code present
;must use a task gate, to preserve stability
	mov ax, 10h
	mov ds, ax			;data segment (we can use variables with their name now)
	push Ten
	call display		;print string (C++ function)
	pop eax
	jmp $
Eleven db 'Segment not present', 10, 0
isr11:
;fault, error code present
	mov ax, 10h
	mov ds, ax			;data segment (we can use variables with their name now)
	push Eleven
	call display	;print string (C++ function)
	pop eax
	jmp $
Twelve db 'Stack fault exception', 10, 0
isr12:
;fault, error code present
	mov ax, 10h
	mov ds, ax			;data segment (we can use variables with their name now)
	push Twelve
	call display	;print string (C++ function)
	pop eax
	jmp $
;Thir1 db '*Error Code:', 0
;Thir2 db ',EIP:', 0
;Thir3 db ',CS:', 0
;Thir4 db ',EFLAGS:', 0
;Thir5 db ',ESP:', 0
;Thir6 db ',SS:', 0
Thirteen db 'General Protection Fault', 10, 0
isr13:
;fault, error code present
	mov ax, 10h
	mov ds, ax			;data segment (we can use variables with their name now)
	push Thirteen
	call display	;print string (C++ function)
	pop eax
;	push Thir1
;	call display
;	pop eax
;	call PrintNumber
;	pop eax
;	push Thir2
;	call display
;	pop eax
;	call PrintNumber
;	pop eax
;	push Thir3
;	call display
;	pop eax
;	call PrintNumber
;	pop eax
;	push Thir4
;	call display
;	pop eax
;	call PrintNumber
;	pop eax
;	push Thir5
;	call display
;	pop eax
;	call PrintNumber
;	pop eax
;	push Thir6
;	call display
;	pop eax
;	call PrintNumber
;	pop eax
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
	call display		;print string (C++ function)
	pop eax
	;check that a reserved bit was not set in the page directory (thats bad)
	mov eax, Code
	and eax, 1000b
	cmp eax, 1000b
	jne .Yay
	push _Fourteen
	call display
	pop eax
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
	call display
	pop eax
	jmp $
Ok:
	push _3Fourteen
	call display
	pop eax
	jmp $

Fifteen db 'Intel reserved interrupt has been called - this is bad', 10, 0
isr15:
;fault, no error code
	mov ax, 10h
	mov ds, ax			;data segment (we can use variables with their name now)
	push Fifteen
	call display		;print string (C++ function)
	pop eax
	jmp $
Sixteen db 'x87 FPU Floating-Point Error', 10, 0
isr16:
;fault, no error code
	mov ax, 10h
	mov ds, ax			;data segment (we can use variables with their name now)
	push Sixteen
	call display		;print string (C++ function)
	pop eax
	jmp $
Seventeen db 'Alignment Check Exception', 10, 0
isr17:
;fault, error code present
	mov ax, 10h
	mov ds, ax			;data segment (we can use variables with their name now)
	push Seventeen
	call display		;print string (C++ function)
	pop eax
	jmp $
Eighteen db 'Machine-Check Exception', 10, 0
isr18:
;abort, no error code
	mov ax, 10h
	mov ds, ax			;data segment (we can use variables with their name now)
	push Eighteen
	call display		;print string (C++ function)
	pop eax
	jmp $
Nineteen db 'SIMD Floating-Point Exception', 10, 0
isr19:
;fault, no error code
	mov ax, 10h
	mov ds, ax			;data segment (we can use variables with their name now)
	push Nineteen
	call display		;print string (C++ function)
	pop eax
	jmp $

EnablePaging:
	mov eax, 0x0100000	;set the base of the page directory (1 MB)
	mov cr3, eax		;time to set the paging bit
	mov eax, cr0
	or eax, 0xE0000000
	mov cr0, eax
.keepgoing
	mov eax, cr0		;time to make sure it worked
	and eax, 0xE0000000
	cmp eax, 0xE0000000
	jne .keepgoing
	sti
	ret

EnableFloppy:
	pusha
	;read data concerning the fdc from BIOS
	lea esi, [0x000FEFC7]
	lea edi, [floppy_parameter]
	mov ecx, floppy_parameter_end - floppy_parameter;	;nunmber of bytes to copy
.nextFlop
	mov al, byte [esi]
	mov [edi], byte al
	inc esi
	inc edi
	loop .nextFlop
	mov al, 0x00
	mov dx, 0x3F2
	out dx, al
	mov al, 0x04
	out dx, al
.waitRes
	mov dx, 0x3F4
	in al, dx
	test al, 0x80
	jne .waitRes
	mov dx, 0x3F5
	in al, dx
	mov bl, al
	xor al, al
	mov dx, 0x3F2
	out dx, al
	cmp bl, 0x90
	je .noCLI
	cli
.noCLI
	mov dx, 0x0A
	mov al, 0x06
	out dx, al
	
	popa
	ret;

WaitFDC:
	pusha
.readyYet
	mov dx, 0x03F4
	in  al, dx
	and al, 0xC0
	cmp al, 0x80
	jne .readyYet
	popa
	ret

RecFDC:
	push dx
.readyYet
	mov dx, 0x3F4
	in al, dx
	and al, 0xD0
	cmp al, 0xD0
	jne .readyYet
	pop dx
	ret	

;floppy disk parameters for cinfigurationo and bla and bla and bla
floppy_parameter:
	steprate_headunload db 0
	headload_ndma db 0
	motor_delay_off db 0	;specified in clock ticks
	bytes_per_sector db 0
	sectors_per_track db 0
	gap_length db 0
	data_length db 0	;used only when bytes per sector equals 0
	format_gap_length db 0
	filler db 0
	head_settle_time db 0	;milliseconds
	motor_start_time db 0	;1/8 seconds
floppy_parameter_end:

FDCCheckIntStat:
	push dx
	mov dx, 0x3F5	;data register
	mov al, 0x08
	call WaitFDC
	out dx, al
	mov dx, 0x3F5
	call RecFDC	;wait until fdc has data ready
	in al, dx	;this should be st0
	push ax
	push eax
	call PrintNumber
	pop eax
	call RecFDC
	in al, dx	;this should be current cylinder	
	pop ax
	pop dx
	ret


	
FDCInt:		;waits for a floppy drive interrupt
	pusha
.time2
	cmp ebx, [BytesDone]
	je .time2
	popa
	ret	

FdcStart:	;starts up the fdc and prepares it to do something
	pusha
	mov ebx, [BytesDone]
	mov dx, 0x3F2
	mov al, 0x1C
	out dx, al
	mov dx, 0x3F7
	xor al, al
	out dx, al
;	call FDCInt
;	mov dx, 0x3F5
;	mov al, 0x08
;	out dx, al
;	call RecFDC
;	call RecFDC
;	call RecFDC
	mov dx, 0x3F2
	mov al, 0x1C
	out dx, al
	;command configure
	mov dx, 0x3F5
	mov al, 0x13
	call WaitFDC
	out dx, al
	mov al, 0
	call WaitFDC
	out dx, al
	mov al, 0x11
	call WaitFDC
	out dx, al
	mov al, 0
	call WaitFDC
	out dx, al
	;specify command
	mov al, 0x03
	call WaitFDC
	out dx, al
	mov al, [steprate_headunload]
	call WaitFDC
	out dx, al
	mov al, [headload_ndma]
	and al, 0xFE
	or al, 1
	call WaitFDC
	out dx, al	
	mov al, 0x07
	call WaitFDC
	out dx, al
	mov al, 0x00
	call WaitFDC
	out dx, al
	;wait 550 ms
	mov bx, 550
	call Delay	
	popa
	ret

FdcEnd:		;shuts down the fdc and prepares it to do nothing
	pusha
	mov dx, 0x3F2
	mov al, 0
	out dx, al
	mov bx, 550
	call Delay
	
	popa
	ret
Address dd 0
Cylinder dd 0
Head dd 0
Sector dd 0
DriveNum dd 0
Yes db 'Thats right', 10, 0
Bla db '!', 0
ReadSector:		;reads one sector from a floppy disk (0x0F03)
	pushad
	mov ebp, esp
	;sub esp, 0x1		;1 byte for local variables
	;activate the appropiate floppy drive motor
	mov eax, [ebp + 12]
	mov [Sector], eax
	mov eax, [ebp + 16]
	mov [DriveNum], eax	;loads all arguments to easy memory (easier to remember what they are)
	mov eax, [ebp + 8]
	mov [Address], eax
	;calculate chs from logical sector from fdc data
	;but not yet
	call FdcStart
	mov ebx, [BytesDone]
	;time to call the read sector command
;	mov dx, 0x3F5
;	mov al, 0x46
;	call WaitFDC
;	out dx, al
;	mov al, 0
;	call WaitFDC
;	out dx, al
;	mov al, 0
;	call WaitFDC
;	out dx, al
;	mov al, 0
;	call WaitFDC
;	out dx, al
;	mov al, 1
;	call WaitFDC
;	out dx, al
;	mov al, 2	;track length / max sector number (which one do i choose)
;	call WaitFDC
;	out dx, al
;	mov al, [gap_length]
;	call WaitFDC
;	out dx, al
;	mov al, 0xFF
;	call WaitFDC
;	out dx, al
;	call FDCInt
;	push Yes
;	call display
;	pop eax
;	call FdcEnd
	mov esp, ebp
	popad
	ret

ReadSectors:		;reads sectors from a floppy disk by making calls to ReadSector
	mov eax, 0
	ret
