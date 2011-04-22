;this is where execution starts
;this will be located at 0x100000 (1MB)
[BITS 32]
[extern display]		;void display(char *chr)
[extern PrintNumber]	;void PrintNumber(unsigned long)
[extern main]			;int main(
[global timer]

global start
start:
  mov esp, stack     ; This points the stack to our new stack area
  jmp skip

; This part MUST be 4byte aligned, so we solve that issue using 'ALIGN 4'
ALIGN 4
mboot:
  ; Multiboot macros to make a few lines later more readable
  MULTIBOOT_PAGE_ALIGN	equ 1<<0
  MULTIBOOT_MEMORY_INFO	equ 1<<1
  MULTIBOOT_AOUT_KLUDGE	equ 1<<16
  MULTIBOOT_HEADER_MAGIC	equ 0x1BADB002
  MULTIBOOT_HEADER_FLAGS	equ MULTIBOOT_PAGE_ALIGN | MULTIBOOT_MEMORY_INFO | MULTIBOOT_AOUT_KLUDGE
  MULTIBOOT_CHECKSUM	equ -(MULTIBOOT_HEADER_MAGIC + MULTIBOOT_HEADER_FLAGS)
  EXTERN code, bss, end

 ; This is the GRUB Multiboot header. A boot signature
  dd MULTIBOOT_HEADER_MAGIC
  dd MULTIBOOT_HEADER_FLAGS
  dd MULTIBOOT_CHECKSUM
    
  ; AOUT kludge - must be physical addresses. Make a note of these:
  ; The linker script fills in the data for these ones!
  dd mboot
  dd code
  dd bss
	dd end
	dd start

Error db 'ERROR: The system was not booted with a multiboot compliant loader', 0
;Success db 'test', 13, 10, 0
set3 db 'Using scancode set 3', 13, 10, 0
bad db 'Using scancode set 1', 13, 10, 0

skip:
;ebx contains the address of an important structure, multiboot_info
	cmp eax, 0x2BADB002
	je .hooray
	push Error
	call display
	pop eax
	jmp $
.hooray
	;mov [bootInfo], ebx	
	;this will be needed if the ebx register is ever touched
	;set up GDT and refresh segments
	lgdt[gdt_desc]
	mov ax, 0x10
	mov ds, ax			;set the segment registers
	mov ss, eax			;and stack
	xor eax, eax
	mov es, eax
	mov fs, eax
	mov gs, eax
	;refresh CS with a far jump
	jmp 0x08:flush2
flush2:
	[extern setupIdt]
	call setupIdt		;return value is stored in eax
	lidt [eax]		;load the idt
	;lidt [idt_desc]	;load the IDT
	;initialize the PIC
	mov al, 00010001b
	out 0x20, al
	out 0xA0, al	;begin initialing master and slave PIC
	mov al, 0x20	;IRQ 0 for master goes to INT 32d
	out 0x21, al	
	mov al, 0x28	;IRQ 0 for slave goes to INT 40d
	out 0xA1, al
	mov al, 00000100b	;slave PIC connected to IRQ 2 of master PIC
	out 0x21, al
	mov al, 0x2		;slave PIC is connected to IRQ2
	out 0xA1, al
	mov al, 1		;Intel, manual EOI
	out 0x21, al
	out 0xA1, al
	mov al, 0	;enable IRQ's
	out 0x21, al	;enable IRQ's
	sti	;enable interrupts
	;setup the clock (irq 0) to have a frequency of about 1000 Hz
	;1193180 / Hz is the number to send through to port 0x40
	mov al, 0x34
	out 0x43, al
	mov al, 0xA9	;lower byte
	out 0x40, al
	mov al, 0x04	;upper byte
	out 0x40, al
	;try to initialize the keyboard to the easiest scan-code set
;	mov al, 0
;	mov [LastResponse], al
;.wait1
;	in al, 0x60
;	and al, 00000010b
;	jnz .wait1
;	mov al, 0xF0
;	out 0x60, al
;.wait2
;	in al, 0x60
;	and al, 00000010b
;	jnz .wait2
;	mov al, 3
;	out 0x60, al
;.wait3
;	mov al, [LastResponse]
;	cmp al, 0xFE
;	je .bad
;	cmp al, 0xFA
;	jne .wait3
;	push set3
;	call display
;	pop eax
;	jmp .good
;.bad
	push bad
	call display
	pop eax
.good
	[extern kernel_end]
	mov eax, kernel_end
	push eax	;kernel size is the second argument
	mov eax, ebx
	;mov eax, [bootInfo]
	;use this instead if ebx is changed
	push eax	;pointer is the first argument
	call main
	pop eax
END:
	nop
	nop
	jmp END

idt_point dd 0
;bootInfo dd 0	
;this is only needed if ebx is modified

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

[global getEIP]
getEIP:
	mov eax, [esp]
	ret

[global EnablePaging]
EnablePaging:
	mov eax, [esp + 4]    ;get the base of the page directory (right above the kernel)
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

save_eax dd 0
debug01 db 13, 10, 'Eax: ', 0
debug02 db ', Ebx: ', 0
debug03 db ', Ecx: ', 0
debug04 db ', Edx: ', 0
debug05 db 13, 10, 'Ebp: ', 0
debug06 db ', Esp: ', 0
debug07 db ', Esi: ', 0
debug08 db ', Edi: ', 0
debug09 db 13, 10, 'Eip: ', 0
debug10 db ', CS: ', 0
debug11 db ', SS: ', 0
debug12 db ', DS: ', 0
debug13 db 'GDTR: ', 0
debug14 db ', IDTR: ', 0
debug15 db ', cr0: ', 13, 10, 0
newline db 13, 10, 0

dump_cpu:
	mov [save_eax], eax
	push debug01
	call display
	pop eax
	mov eax, [save_eax]
	push eax
	call PrintNumber
	pop eax
	push debug02
	call display
	pop eax
	mov eax, ebx
	push eax
	call PrintNumber
	pop eax
	push debug03
	call display
	pop eax
	mov eax, ecx
	push eax
	call PrintNumber
	pop eax
	push debug04
	call display
	pop eax
	mov eax, edx
	push eax
	call PrintNumber
	pop eax
	push debug05
	call display
	pop eax
	mov eax, ebp
	push eax
	call PrintNumber
	pop eax
	push debug06
	call display
	pop eax
	mov eax, esp
	push eax
	call PrintNumber
	pop eax
	push debug07
	call display
	pop eax
	mov eax, esi
	push eax
	call PrintNumber
	pop eax
	push debug08
	call display
	pop eax
	mov eax, edi
	push eax
	call PrintNumber
	pop eax
	push debug09
	call display
	pop eax
	mov eax, [esp]
	push eax
	call PrintNumber
	pop eax
	push debug10
	call display
	pop eax
	mov eax, cs
	push eax
	call PrintNumber
	pop eax
	push debug11
	call display
	pop eax
	mov eax, ss
	push eax
	call PrintNumber
	pop eax
	push debug12
	call display
	pop eax
	mov eax, ds
	push eax
	call PrintNumber
	pop eax
;	push debug13
;	call display
;	pop eax
;	mov eax, cs
;	push eax
;	call PrintNumber
;	pop eax
;	push debug14
;	call display
;	pop eax
;	xor eax, eax
;	sgdt eax
;	push eax
;	call PrintNumber
;	pop eax
;	push debug15
;	call display
;	pop eax
;	xor eax, eax
;	sidt eax
;	push eax
;	call PrintNumber
;	pop eax
	push newline
	call display
	pop eax
	mov eax, [save_eax]
ret

[global irqM0]
[global irqM1]
[global irqM2]
[global irqM3]
[global irqM4]
[global irqM5]
[global irqM6]
[global irqM7]
[global isr0]
[global isr1]
[global isr2]
[global isr3]
[global isr4]
[global isr5]
[global isr6]
[global isr7]
[global isr8]
[global isr9]
[global isr10]
[global isr11]
[global isr12]
[global isr13]
[global isr14]
[global isr15]
[global isr16]
[global isr17]
[global isr18]
[global isr19]

timer dd 0	;a measure of time since the system started

irqM0:
	push ax
	inc dword [timer]
	;manual EOI before the interrupt has ended
	mov al, 0x20
	out 0x20, al
	pop ax
	iret

[global Delay]
Delay:
	push eax
	push ebx
	mov eax, [timer]
	add eax, [esp + 12]		;eax = delay + time
.wait
	mov ebx, [timer]
	cmp eax, ebx
	jg .wait
	pop ebx
	pop eax
	ret
	

test_key db 13, 0
PauseKey db 0	;becomes nonzero when any key is pressed
[global WaitKey]
WaitKey:
	push ax
	mov al, 0
	mov [PauseKey], al
.wait
	cmp al, [PauseKey]
	je .wait
	pop ax
	ret

ScanSet db 0, '1234567890-=', 8, 9, 'qwertyuiop[]', 0, 0, 'asdfghjkl;', 0x27, '`', 0, '\zxcvbnm,./', 0, '*', 0, ' ', 0, 0, 0, 0, 0, 0 ,0 ,0 ,0, 0, 0, 0, 0, '789-456+1230', 0
ShiftSet db 0,'!@#$%^&*()_+', 8, 9, 'QWERTYUIOP{}', 0, 0, 'ASDFGHJKL:"~', 0, '|ZXCVBNM<>?', 0, '*', 0, ' ', 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, '789-456+1230', 0
;these arrays will be used to retrieve ASCII information from the keyboard
convert db 0, 0, 0	;this is used to store the converted scan code
Newline db 13, 10, 0	;this is needed because when you press enter, two bytes need to be sent for it to work properly
Buffer db 0,0,0,0,0,0	;used to process multi byte scancodes
Shift db 0		;determines if a shift is currently being held down
Alt db 0		;determines if an alt key is being held down
Ctrl db 0		;same thing for ctrl key
LastResponse db 0	;stores the last response from a command (not a scancode)
;OneByte db ' One byte code*', 0
;TwoByte db 'Two byte code*', 0
SixByte db 'Six byte code*', 0
;FourByte db 'Four byte code*', 0

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
;	push eax
;	call PrintNumber
;	pop eax
	cmp al, 0xFE
	jne .notFE
	mov [LastResponse], al
	jmp .notFA
.notFE
	cmp al, 0xFA
	jne .notFA
	mov [LastResponse], al
.notFA
;	push OneByte
;	call display
;	pop ebx
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
;	push TwoByte
;	call display
;	pop ebx
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
	cmp bl, 0xE1	;afaik, pause/break is the only 6 byte scancode
	je irqM1_end
;@handle 4 byte scancodes
;	push FourByte
;	call display
;	pop ebx
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
	mov al, 1
	mov [PauseKey], al
;	push SixByte
;	call display
;	pop ebx
;	jmp $
	;done handling 6 byte scancode(s)
	xor ebx, ebx
	mov [Buffer], ebx
	mov [Buffer + 4], bl	;this should clear 6 bytes the easy way
	jmp irqM1_end

irqM1_full_code:	;when the last byte of a scancode has been read


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
	;chech for cpuid
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

SECTION .bss
    resb 8192               ;8 KB for the stack, doesn't need to be initialized
stack:
