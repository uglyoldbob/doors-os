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
[global __Z14EnableKeyboardv]	;unsigned long int EnableKeyboard(void)
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
	mov al, 0x0
	mov dx, 0x3F2
	out dx, al		;reset the floppy drive controller and turn off all motors
	call enableA20
	;re-enable interrupts
	mov al, 0	;enable IRQ's
	out 0x21, al	;enable IRQ's
	;setup the clock (irq 0) to havbe a frequency of about 1000 Hz
	;1193180 / Hz is the number to send through to port 0x40
	mov al, 0x34
	out 0x43, al
	mov al, 0xA9	;lower byte
	out 0x40, al
	mov al, 0x04	;upper byte
	out 0x40, al 
	mov al, 30
	mov ah, 0x9B
	mov bl, 0xB6
	;setup the realtime clock (irq 8) to have a frequency of 1KHz
	;disable interrupts and nmi (using bit 7 of port 0x70
	;dont do this yet, wait until keyboard and floppy disk drivers are complete
	call MakeNoise
	start:
	call __main
	call _main 		;call int main(void), which is located in our C++ code
	cmp eax, 0
	je Yay
	push Bad
	call __Z7displayPc		;print string (C++ function)
	pop eax
	jmp Yay2
Yay:
	push Good
	call __Z7displayPc		;print string (C++ function)
	pop eax
	Yay2:
	call __atexit
	jmp $

MakeNoise:
	;low byte	-bl
	;high byte	-ah
	;number of clock cycles to last - al
	;time to test some new code
	;0x1234DD / 100 = 2E9B
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
idt40:	;MASTER IRQ 8
	dw (irqM8-$$+0x0900) & 0xFFFF			;get the lower word for the offset
	dw 0x08		;the code segment for our segment
	db 0			;a zero byte
	db 10001110b	;flags byte
	dw (irqM8-$$+0x0900) >> 16			;the upper byte of the offset

;20 - 31 are reserved, 32-39 used for master IRQ 0 - 7, 40 - 47 slave IRQ 0 - 7, 
	;48-255 are usable for anything
idt_end:
idt_desc:				;IDT descriptor
	dw idt_end - idt - 1	;limit
	dd idt		;address for idt

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
BadKey db 'Something terrible happened to the keyboard driver!', 13, 10, 0
Temp db '?', 0	;this is used to store what will be displayed for the key that was just pressed (0 means nothing will be displayed)
Newline db 13, 10, 0	;this is needed because when you press enter, two bytes need to be sent for it to work properly
Buffer db 0,0,0,0,0,0	;used to process multi byte scancodes
Shift db 0		;determines if a shift is currently being held down
irqM1:
	pusha
	xor eax, eax
	in al, 0x60
	cmp al, 00
	jne .NotTooMany
	;beep or something
	jmp .end
.ShiftNotPressed
	mov ebx, ScanSet
	jmp .doneShifting
.ShiftPressed
	mov ebx, ShiftSet
	jmp .doneShifting
.doneShifting
	add ebx, eax
	sub ebx, 1
	cmp al, 0x1C
	jne .notEnter
	push Newline
	call __Z7displayPc
	pop eax
	jmp .end
.notEnter
	;check for shift keys (either) and update the shift variable if necessary
	cmp al, 0x2A
	jne .shift1
	mov al, 1
	mov [Shift], al
	jmp .end
.shift1
	cmp al, 0x36
	jne .notShift
	mov al, 1
	mov [Shift], al
	jmp .end
.notShift
	mov al, [ebx]	;figure out how to use pointers with asm
	mov [Temp], al
	push Temp
	call __Z7displayPc
	pop eax
	jmp .end
.NotTooMany	;good the user is not pressing too many keys\
	cmp al, 0x52
	jl .N1
	cmp al, 0x57
	je .N1		;these two codes are skipped by the above jl .n1 statement
	cmp al, 0x58
	je .N1
	mov bl, [Buffer]
	cmp bl, 0
	jne .N1		;an advanced scancode is currently being recieved
	mov bl, [Shift]
	cmp bl, 0
	je .ShiftNotPressed
	jmp .ShiftPressed
.N1
	;provides additional processing for multi-byte scancodes
	mov bl, [Buffer]
	cmp bl, 0
	jne .2
	;ok we have recieved the first byte of a special scancode, it might be a 1byte scancode for keys released
	cmp al, 0xE0
	jne .1
	je .1act
	cmp al, 0xE1
	jne .1
	je .1act
	;cmp al, 0x5B	why is this here?
.1act
	mov [Buffer], al
	jmp .end
.1	;right now there is nothing to do when a 1-byte scancode is released
	cmp al, 0xAA
	je .ShiftRel
	cmp al, 0xB6
	je .ShiftRel
	jmp .1KeyRel	;a shift key was not released
.ShiftRel	
	xor bl, bl	;a shift key has been released
	mov [Shift], bl
.1KeyRel
	mov bl, 0
	mov [Buffer], bl
	jmp .end	

.end:
	;manual EOI before the interrupt has ended
	mov al, 0x20
	out 0x20, al
	popa
	iret

.2
	;we have recieved the second byte of a multi-byte scancode
	;there are some two byte scancodes out there
	mov bl, [Buffer + 1]
	cmp bl, 0
	jne .3
	mov [Buffer + 1], al
	;time to check for valid scancodes 
	;l win, r win, r alt, menu, r ctrl, print screen repeat, ins, home, pgup, del, end, pgdwn, up, down, left, right, numpad enter, numpad /
	;check for bytes that are not two byte scancodes
	mov bl, [Buffer]
	cmp bl, 0xE1
	je .end		;dont process yet (its the 6 byte scancode)
	mov bl, [Buffer + 1]
	cmp bl, 0x2A
	je .end
	cmp bl, 0xB7
	je .end
	;the following scancodes are only longer if the internal numlock is active?
	cmp al, 0xDB
	je .end
	cmp al, 0xDC
	je .end
	cmp al, 0xDD
	je .end
	cmp al, 0xD2
	je .end
	cmp al, 0xD3
	je .end
	cmp al, 0xD1
	je .end
	cmp al, 0xD0
	je .end
	cmp al, 0xC7
	je .end
	cmp al, 0xCF
	je .end
	cmp al, 0xC9
	je .end
	cmp al, 0xCB
	je .end
	cmp al, 0xC8
	je .end
	cmp al, 0xCD
	je .end
.numlockinactive
	;processing for two byte scancodes
.done2
	mov bl, 0
	mov [Buffer], bl
	mov [Buffer + 1], bl
	jmp .end
.3
	mov bl, [Buffer + 2]
	cmp bl, 0
	jne .4
	mov [Buffer + 2], al
	jmp .end
.4
	mov bl, [Buffer + 3]
	cmp bl, 0
	jne .5
	mov [Buffer + 3], al
	cmp al, 0xE1		;this is the break key
	je .5
	;processing for four byte scancodes
	xor bx, bx
	mov [Buffer], bl
	mov [Buffer + 1], bl
	mov [Buffer + 2], bl
	mov [Buffer + 3], bl
	jmp .end
.5
	mov bl, [Buffer + 4]
	cmp bl, 0
	jne .6
	mov [Buffer + 4], al
	jmp .end
.6
	cmp al, 0xC5
	je .noError
	push BadKey
	call __Z7displayPc
	pop eax
	xor bx, bx
	mov [Buffer], bl
	mov [Buffer + 1], bl
	mov [Buffer + 2], bl
	mov [Buffer + 3], bl
	mov [Buffer + 4], bl
	mov [Buffer + 5], bl
	jmp .end
.noError
	;handle break key
	jmp .end
	
IRQM2 db 'IRQ2', 13, 10, 0
irqM2:
	push eax
	;manual EOI before the interrupt has ended
	push IRQM2
	call __Z7displayPc
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
	call __Z7displayPc
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
	call __Z7displayPc
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
	call __Z7displayPc
	pop eax
	mov al, 0x20
	out 0x20, al
	pop eax
	iret

Mode db 0	;defines what the interrupt should be expecting to do when called
IRQM6 db 'FDC has fired an interrupt!', 10, 13, 0
irqM6:
;this is IRQ 6 from the master PIC
	push IRQM6
	call __Z7displayPc
	pop eax
	;inc byte [DoneYet]
	;manual EOI before the interrupt has ended
	mov al, 0x20
	out 0x20, al
	;now return to wherever execution was before this interrupt
	iret

IRQM7 db 'IRQ7', 13, 10, 0
irqM7:
	push eax
	;manual EOI before the interrupt has ended
	push IRQM7
	call __Z7displayPc
	pop eax
	mov al, 0x20
	out 0x20, al
	pop eax
	iret

IRQM8 db 'IRQ8', 13, 10, 0
irqM8:
	push eax
	;manual EOI before the interrupt has ended
	push IRQM8
	call __Z7displayPc
	pop eax
	mov al,20h    ;EOI command
	out 0xA0,al   ;Sending the command to the slave PIC
	out 0x20,al    ;Sending the command to the master PIC
	iret

Code dd 0	;this stores any error code that needs to be examined in the following routines
Zero db 'Divide by zero error!', 10, 0
isr0:
;fault, no error code
;display message, stop, because returning brings it back
	mov ax, 10h
	mov ds, ax			;data segment (we can use variables with their name now)
	push Zero
	call __Z7displayPc		;print string (C++ function)
	pop eax
	jmp $
One db 'Debug exception', 10, 0
isr1:
;trap or fault, no error code
;examine DR6 and other debug registers to determine type (not necessary though)
	mov ax, 10h
	mov ds, ax			;data segment (we can use variables with their name now)
	push One
	call __Z7displayPc		;print string (C++ function)
	pop eax
	jmp $
Two db 'NMI Interrupt', 10, 0
isr2:
;interrupt, no error code
;hmm what to do? i dont know...
	mov ax, 10h
	mov ds, ax			;data segment (we can use variables with their name now)
	push Two
	call __Z7displayPc		;print string (C++ function)
	pop eax
	jmp $
Three db 'Breakpoint', 10, 0
isr3:
;trap, no error code
	mov ax, 10h
	mov ds, ax			;data segment (we can use variables with their name now)
	push Three
	call __Z7displayPc		;print string (C++ function)
	pop eax
	jmp $
Four db 'Overflow', 10, 0
isr4:
;trap, no error code
	mov ax, 10h
	mov ds, ax			;data segment (we can use variables with their name now)
	push Four
	call __Z7displayPc		;print string (C++ function)
	pop eax
	jmp $
Five db 'Bounds range exceeded', 10, 0
isr5:
;fault, no error code
	mov ax, 10h
	mov ds, ax			;data segment (we can use variables with their name now)
	push Five
	call __Z7displayPc		;print string (C++ function)
	pop eax
	jmp $
Six db 'Invalid opcode', 10, 0
isr6:
;fault, no error code
	mov ax, 10h
	mov ds, ax			;data segment (we can use variables with their name now)
	push Six
	call __Z7displayPc		;print string (C++ function)
	pop eax
;00 10 B8 66 8e 00 10 b8 00 10 b8 66 00 0a 65 64 6f 63 70 6f
;   ?? ?? f  ??    ?? ??    ?? ?? f     ?? e  d  o  c  p 
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
	pop eax
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
	pop eax
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

;when all of the print statements are uncommented, the program freezes for an unknown reason
	;maybe alignement related
;these are for int 13 (general protection fault)
Thir1 db '*Error Code:', 0
Thir2 db ',EIP:', 0
Thir3 db ',CS:', 0
Thir4 db ',EFLAGS:', 0
Thir5 db ',ESP:', 0
Thir6 db ',SS:', 0
Thirteen db 'General Protection Fault', 10, 0
isr13:
;fault, error code present
	mov ax, 10h
	mov ds, ax			;data segment (we can use variables with their name now)
	push Thirteen
	call __Z7displayPc	;print string (C++ function)
	pop eax
	push Thir1
	call __Z7displayPc
	pop eax
	call __Z11PrintNumberm
	pop eax
	push Thir2
	call __Z7displayPc
	pop eax
	call __Z11PrintNumberm
	pop eax
	push Thir3
	call __Z7displayPc
	pop eax
	call __Z11PrintNumberm
	pop eax
	push Thir4
	call __Z7displayPc
	pop eax
	call __Z11PrintNumberm
	pop eax
	push Thir5
	call __Z7displayPc
	pop eax
	call __Z11PrintNumberm
	pop eax
	push Thir6
	call __Z7displayPc
	pop eax
	call __Z11PrintNumberm
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
	pop eax
	;check that a reserved bit was not set in the page directory (thats bad)
	mov eax, Code
	and eax, 1000b
	cmp eax, 1000b
	jne .Yay
	push _Fourteen
	call __Z7displayPc
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
	call __Z7displayPc
	pop eax
	jmp $
Ok:
	push _3Fourteen
	call __Z7displayPc
	pop eax
	jmp $

Fifteen db 'Intel reserved interrupt has been called - this is bad', 10, 0
isr15:
;fault, no error code
	mov ax, 10h
	mov ds, ax			;data segment (we can use variables with their name now)
	push Fifteen
	call __Z7displayPc		;print string (C++ function)
	pop eax
	jmp $
Sixteen db 'x87 FPU Floating-Point Error', 10, 0
isr16:
;fault, no error code
	mov ax, 10h
	mov ds, ax			;data segment (we can use variables with their name now)
	push Sixteen
	call __Z7displayPc		;print string (C++ function)
	pop eax
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
	pop eax
	jmp $
Nineteen db 'SIMD Floating-Point Exception', 10, 0
isr19:
;fault, no error code
	mov ax, 10h
	mov ds, ax			;data segment (we can use variables with their name now)
	push Nineteen
	call __Z7displayPc		;print string (C++ function)
	pop eax
	jmp $

__Z12EnablePagingv:
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
	ret

__Z12EnableFloppyv:

	ret;


_FDCRecvByte:
	mov dx, 0x3F4
.loop0:
	in al, dx
	test al, 0xC0
	jnz .end
	;hlt
	jmp .loop0
.end:
	mov dx, 0x3F5
	in al, dx
	ret

_FDCSendByte:
	push bp
	mov bp, sp
	mov dx, 0x3F4
.loop0:
	in al, dx
	test al, 0x80
	jnz .end
	;hlt
	jmp .loop0
.end:
	mov dx, 0x3F5
	mov al, [bp + 4]
	out dx, al
	pop bp
	ret

_waitFDCDone:
	test [DoneYet], byte 0xff
	jnz .end
	;hlt
	jmp _waitFDCDone
.end:
	mov [DoneYet], byte 0
	ret

Address dd 0
Sector dd 0
DriveNum dd 0
DoneYet db 0		;this is what determines if we are done
Yes db 'Thats right', 10, 0
Bla db '!', 0
__Z10ReadSectormmh:		;reads one sector from a floppy disk (0x0F03)
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
	;reset the floppy disk controller to put it into a known state
	mov al, 0x14
	mov edx, [DriveNum]
	or al, dl		;load the drivenum into al
	mov dx, 0x3F2
	out dx, al
	;wait until the controller is ready to accept commands
.readyYet
	mov dx, 0x03F4
	in  al, dx
	and al, 0xC0
	cmp al, 0x80
	jne .readyYet
	
	push Yes
	call __Z7displayPc
	pop eax
	;wonder if it did anything?
	mov esp, ebp
	popad
	ret

__Z11ReadSectorsmmmh:		;reads sectors from a floppy disk by making calls to ReadSector
	mov eax, 0
	ret
