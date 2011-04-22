;this is where execution starts
;this will be located at 0x100000 (1MB)
[BITS 32]
[extern display]		;void display(char *chr)
[extern PrintNumber]	;void PrintNumber(unsigned long)
[extern main]			;int main(
[extern memcopy]
[extern get_sys_tasks]
[global timer]
[global syscall1]

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

skip:
;ebx contains the address of an important structure, multiboot_info
	cmp eax, 0x2BADB002
	je .hooray
	push Error
	call display
	pop eax
	jmp $
.hooray
	cli		;interrupt flag should already be cleared, but i'll disable them as an extra precaution
	;mov [bootInfo], ebx	
	;this will be needed if the ebx register is ever touched
	;set up GDT and refresh segments
	lgdt [gdt_desc]
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
	hlt
	jmp END

idt_point dd 0
bootInfo dd 0	
;this is only needed if ebx is modified

;TODO: change the location of the GDT (with TSS that needs to be added) to a shared page
;malloc(0x1000)
;copy to this address
;it will be the shared page
;update all GDT information
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

[global inportb]
inportb:
	mov edx, [esp + 4]
	xor eax, eax
	in al, dx
	ret

[global outportb]
outportb:
	push edx
	mov eax, [esp + 8]
	mov edx, [esp + 12]
	out dx, al
	pop edx
	ret

[global inportw]
inportw:
	mov edx, [esp + 4]
	xor eax, eax
	in ax, dx
	ret

[global outportw]
outportw:
	push edx
	mov eax, [esp + 8]
	mov edx, [esp + 12]
	out dx, ax
	pop edx
	ret

[global getEIP]
getEIP:
	mov eax, [esp]
	ret

[global multi_tss_begin]

multi_start:			;address "0"
multi_gdt:                    ; Address for the GDT
multi_gdt_null:               ; Null Segment
	dd 0
	dd 0
multi_gdt_code:               ; Code segment, read/execute, nonconforming
	dw 0xFFFF
	dw 0x0000
	db 0
	db 10011010b	;non-system descriptor (bit 4)
	db 11001111b
	db 0
multi_gdt_data:               ; Data segment, read/write
	dw 0xFFFF
	dw 0x0000
	db 0
	db 10010010b	;non system descriptor (bit 4)
	db 11001111b
	db 0
multi_gdt_tss:
	dw multi_tss_end - multi_tss_begin + 1
	dw multi_tss_begin - multi_start
	db 0
	db 10001001b
	db 00000000b
	db 0
multi_gdt_tss2:
	dw multi_tss2_end - multi_tss2_begin + 1
	dw multi_tss2_begin - multi_start
	db 0
	db 10001001b
	db 00000000b
	db 0
multi_gdt_end:				; Used to calculate the size of the GDT
multi_gdt_desc:				; The GDT descriptor
	dw multi_gdt_end - multi_gdt - 1	; Limit (size)
	dd multi_gdt - multi_start	; Address of the GDT
;the first TSS
multi_tss_begin:
	tss_previous dw 0
	dw 0	;reserved
	tss_esp0 dd 0
	tss_ss0 dw 0
	dw 0	;reserved
	tss_esp1 dd 0
	tss_ss1 dw 0
	dw 0	;reserved
	tss_esp2 dd 0
	tss_ss2 dw 0
	dw 0	;reserved
	tss_cr3 dd 0
	tss_eip dd 0
	tss_eflags dd 0
	tss_eax dd 0
	tss_ecx dd 0
	tss_edx dd 0
	tss_ebx dd 0
	tss_esp dd 0
	tss_ebp dd 0
	tss_esi dd 0
	tss_edi dd 0
	tss_es dw 0
	dw 0
	tss_cs dw 0
	dw 0
	tss_ss dw 0
	dw 0
	tss_ds dw 0
	dw 0
	tss_fs dw 0
	dw 0
	tss_gs dw 0
	dw 0
	tss_ldtsegment
	dw 0
	tss_trapflag dw 0	;the lowest bit is the debug trapflag
						;all else is reserved
	tss_iobaseaddress dw 0
		;end of interrupt redirection bitmap
		;beginning of I/O permission bitmap
		;relative to the base of the TSS
multi_tss_end:
;the second TSS
multi_tss2_begin:
	tss2_previous dw 0
	dw 0	;reserved
	tss2_esp0 dd 0
	tss2_ss0 dw 0
	dw 0	;reserved
	tss2_esp1 dd 0
	tss2_ss1 dw 0
	dw 0	;reserved
	tss2_esp2 dd 0
	tss2_ss2 dw 0
	dw 0	;reserved
	tss2_cr3 dd 0
	tss2_eip dd 0
	tss2_eflags dd 0
	tss2_eax dd 0
	tss2_ecx dd 0
	tss2_edx dd 0
	tss2_ebx dd 0
	tss2_esp dd 0
	tss2_ebp dd 0
	tss2_esi dd 0
	tss2_edi dd 0
	tss2_es dw 0
	dw 0
	tss2_cs dw 0
	dw 0
	tss2_ss dw 0
	dw 0
	tss2_ds dw 0
	dw 0
	tss2_fs dw 0
	dw 0
	tss2_gs dw 0
	dw 0
	tss2_ldtsegment
	dw 0
	tss2_trapflag dw 0	;the lowest bit is the debug trapflag
						;all else is reserved
	tss2_iobaseaddress dw 0
		;end of interrupt redirection bitmap
		;beginning of I/O permission bitmap
		;relative to the base of the TSS
multi_tss2_end:
multi_end:

[global setup_multi_gdt]
setup_multi_gdt:
	push multi_end - multi_start
	push multi_start
	push 0
	call memcopy
	pop eax
	pop eax
	pop eax
	lgdt [multi_gdt_desc - multi_start]
	jmp 0x08:flush3
flush3:

	mov eax, 0x18
	ltr ax

	ret



;[extern enter_spinlock]
;[extern leave_spinlock]
[global test_and_set]	;unsigned int test_and_set (unsigned int new_value, unsigned int *lock_pointer);
											;
test_and_set:
	mov eax, [esp + 4]	;eax = new_value
	mov edx, [esp + 8]	;edx = lock_pointer
	lock xchg [edx], eax			;swap *lock_pointer and new_value
	cmp [edx], eax
	je test_and_set	;only return of the values are different
	ret								;return the old value of *lock_pointer

;most interruptable
SL_BLANK dd 0
SL_MEM_MNG dd 1
SL_IRQ1 dd 2
SL_MESSAGE dd 3
;least interruptable
;can go down the table, not up

[global getCR3]
getCR3:
	mov eax, cr3
	ret

[global EnablePaging]
EnablePaging:
	mov eax, [esp + 4]    ;get the base of the page directory (right above the kernel)
	mov cr3, eax		;time to set the paging bit
	mov eax, cr0		;also set the cache disable bit
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
[global irqM8]
[global irqM9]
[global irqM10]
[global irqM11]
[global irqM12]
[global irqM13]
[global irqM14]
[global irqM15]
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

[global timer]
timer dd 0	;a measure of time since the system started
[extern next_state]	;this loads the TSS information for the next task into the unused system TSS
					;it also stores the TSS information of the previous task
[global enable_multi]

previous_tss db 2	;defines which tss holds the previous task
previous dd 0		;is the unused TSS a valid previous task?
task_timer db 20;	;this is the task timer
enable_multi db 0;	;should we try to multitask yet?

[global get_current_tss]
get_current_tss:
	cmp BYTE [previous_tss], 1
	je .2
	mov eax, multi_tss_begin - multi_start
	jmp .1
.2
	mov eax, multi_tss2_begin - multi_start
.1
	ret

irqM0:	inc dword [timer]
	dec BYTE [task_timer]
	;manual EOI before the interrupt has ended	
	push ax			;save ax
	mov al, 0x20
	out 0x20, al
	pop ax			;restore ax
	cmp BYTE [enable_multi], 0
	je .noSwitch
	cmp BYTE [task_timer], 0
	je .switch
	;check to see if it is time for a task switch
	jmp .end
.noSwitch
	mov BYTE [task_timer], 20
	jmp .end
.switch
	cli
	mov BYTE [task_timer], 20
	push eax	;store eax for after the next_state call
	cmp BYTE [previous_tss], 2
	je .2
	mov eax, multi_tss_begin - multi_start
	push eax
	mov BYTE [previous_tss], 2
	jmp .1
.2
	mov eax, multi_tss2_begin - multi_start
	push eax
	mov BYTE [previous_tss], 1
.1
	call get_sys_tasks
	push eax
	xor eax, eax
	mov ax, [previous]
	push eax
	call next_state	;takes three arguments
	pop eax
	pop eax	;pop arguments off of the stack
	pop eax
	pop eax			;restore eax
	;either this is the first task switch and the previous tss is invalid (in which case the data was not used)
	;or this was not the first task switch (data in the tss was used)
	;either way, once the hardware performs the task switch, the "previous" task will contain information about the current task
	;jump to the TSS that was just loaded (previous task will point to the currently executing task)
	push eax
	mov eax, 1
	mov [previous], eax
	pop eax
	cmp BYTE [previous_tss], 2
	je .2exec
	sti
	jmp 0x20:00
	jmp .1exec
.2exec
	sti
	jmp 0x18:00
.1exec
	;when you return to this task, it will iret and continue normally
	;hoepfully this will not cause problems with irq 0
.end
	iret

[global Delay]
Delay:	;delays for some number of irq0 firings (each of which are about 1 millisecond)
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

;this has to match what is found in sys/syscalls.h
SYS_close	equ	1 ;1
SYS_execve	equ	2 ;3
SYS_exit	equ	3 ;0
SYS_fork	equ	4 ;0
SYS_fstat	equ	5 ;2
SYS_getpid	equ	6 ;0
SYS_isatty	equ	7 ;1
SYS_kill	equ	8 ;2
SYS_link	equ	9 ;2
SYS_lseek	equ	10;3
SYS_open	equ	11;Variable number of arguments
SYS_read	equ	12;3
SYS_sbrk	equ	13;1
SYS_stat	equ	14;2
SYS_time	equ	15;2
SYS_times	equ	16;1
SYS_unlink	equ	17;1
SYS_wait	equ	18;1
SYS_write	equ	19;3

SYS_MAX		equ	20

;newline db 13, 10, 0

syscall1:
	;check for syscalls that have no arguments
	cmp eax, SYS_exit
	je .exit
	cmp eax, SYS_fork
	je .fork
	cmp eax, SYS_getpid
	je .getpid
	jmp .one
.exit	;exit the task that called this
.idle
	hlt
	jmp .idle
.fork
	mov eax, 0xFFFFFFFF	;return error for now
	iret
.getpid
	mov eax, 1
	iret
.one	;one argument
	cmp eax, SYS_sbrk
	je .sbrk
	jmp .two
.sbrk
	push ebx
	call PrintNumber
	pop ebx
	push newline
	call display
	pop eax
	mov eax, 0xFFFFFFFF
	iret
.two
	iret

;the IRQ handler for the keyboard will convert scancodes and then place the code into a buffer
;the buffer will have a start and length
;codes will be pulled from the bottom of the buffer

[global getResponse] ;waits for an retrieves a byte response from the keyboard
getResponse:
	push ecx
	mov ecx, NumKeyInts
.waitForIt
	cmp ecx, [NumKeyInts]
	je .waitForIt
	mov eax, [LastResponse]
	mov ecx, 0
	mov [LastResponse], ecx
	pop ecx
	ret

[extern handleScancode]
[global num_elements_used]
[global code_buffer]
num_elements_used dd 0
code_buffer dd 1, 2, 3, 4, 5, 6

LastResponse db 0	;stores the last response from a command (not a scancode)
NumKeyInts dd 0

;0xE0 0x2A 0xE0, 0x53
irqM1:
	pusha
	xor eax, eax
	in al, 0x60
	;check for codes that don't qualify as the last response
	mov [LastResponse], al
	push eax
	call handleScancode
	pop eax
	mov bl, al	;save retrieved byte
	in  al, 61h
	mov ah, al	;Save keyboard status
	or  al, 80h	;Disable
	out 61h, al
	mov al, ah	;Enable (If it was disabled at first, you wouldn't
	out 61h, al	; be doing this anyway :-)
	mov al, bl	;restore byte recieved
	inc dword [NumKeyInts]

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

FloppyTimeout db 'Timeout waiting for floppy drive', 10, 13, 0

[global WaitFloppyInt]
WaitFloppyInt:
	push ebx
	mov eax, [BytesDone]
	mov ebx, [timer]
	add ebx, 0x200
.notThereYet
	cmp ebx, [timer]
	jl .error
	cmp eax, [BytesDone]
	je .notThereYet
	pop ebx
	mov eax, 0	;indicate success
	ret
.error
	;push FloppyTimeout
	;call display
	;pop eax
	pop ebx
	mov eax, 0xFFFFFFFF	;-1 indicates failure
	ret
	
BytesDone dd 0	;the number of times the interrupt has been fired
IRQM6 db 'FDC has fired an interrupt!', 10, 13, 0
irqM6:
;this is IRQ 6 from the master PIC
	push ax
	;determine what this means
	inc dword [BytesDone]
;	push IRQM6
;	call display
;	pop eax
	;now return to wherever execution was before this interrupt
	mov al, 0x20
	out 0x20, al
	pop ax
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

IRQM8 db 'IRQ8', 13, 10, 0
irqM8:
	push eax
	;manual EOI before the interrupt has ended required for both master and slave PIC	
	push IRQM8
	call display
	pop eax
	mov al, 0x20
	out 0x20, al
	out 0xA0, al
	pop eax
	iret

IRQM9 db 'IRQ9', 13, 10, 0
irqM9:
	push eax
	;manual EOI before the interrupt has ended required for both master and slave PIC	
	push IRQM9
	call display
	pop eax
	mov al, 0x20
	out 0x20, al
	out 0xA0, al
	pop eax
	iret

IRQM10 db 'IRQ10', 13, 10, 0
irqM10:
	push eax
	;manual EOI before the interrupt has ended required for both master and slave PIC	
	push IRQM10
	call display
	pop eax
	mov al, 0x20
	out 0x20, al
	out 0xA0, al
	pop eax
	iret

IRQM11 db 'IRQ11', 13, 10, 0
irqM11:
	push eax
	;manual EOI before the interrupt has ended required for both master and slave PIC	
	push IRQM11
	call display
	pop eax
	mov al, 0x20
	out 0x20, al
	out 0xA0, al
	pop eax
	iret

IRQM12 db 'IRQ12', 13, 10, 0
irqM12:
	push eax
	;manual EOI before the interrupt has ended required for both master and slave PIC	
	push IRQM12
	call display
	pop eax
	mov al, 0x20
	out 0x20, al
	out 0xA0, al
	pop eax
	iret

IRQM13 db 'IRQ13', 13, 10, 0
irqM13:
	push eax
	;manual EOI before the interrupt has ended required for both master and slave PIC	
	push IRQM13
	call display
	pop eax
	mov al, 0x20
	out 0x20, al
	out 0xA0, al
	pop eax
	iret

[global HD_INTS]
HD_INTS dd 0	;number of times the Hard drive controller IRQ has fired
IRQM14 db 'IRQ14', 13, 10, 0
irqM14:
	push eax
	inc DWORD [HD_INTS]
	push IRQM14
	call display
	pop eax
	;manual EOI before the interrupt has ended required for both master and slave PIC
	mov al, 0x20
	out 0x20, al
	out 0xA0, al
	pop eax
	iret

IRQM15 db 'IRQ15', 13, 10, 0
irqM15:
	push eax
	push IRQM15
	call display
	pop eax
	;manual EOI before the interrupt has ended required for both master and slave PIC
	mov al, 0x20
	out 0x20, al
	out 0xA0, al
	pop eax
	iret




Code dd 0	;this stores any error code that needs to be examined in the following routines
Zero db 'Divide by zero error!', 10, 0
isr0:
;fault, no error code
;display message, stop, because returning brings it back
	mov ax, 0x10
	mov ds, ax			;data segment (we can use variables with their name now)
	push Zero
	call display		;print string (C++ function)
	pop eax
	jmp $

One db 'Debug exception, ', 0
sinStep db 'Single Step, ', 0
regAcc db 'Debug register was accessed, ', 0
task db 'Trap flag for task is set,' , 0
breakpoint db 'Breakpoint hit,' , 0
number dd 0	;counts the number of times this is called
isr1:
;trap or fault, no error code
;examine DR6 and other debug registers to determine type
	push eax
	inc dword [number]
	push One
	call display
	pop eax
	mov eax, dr6
	test eax, 0x4000
jz .notSingleStep
	push sinStep
	call display
	pop eax
.notSingleStep
	test eax, 0x2000
	jz .notregAcc
	push regAcc
	call display
	pop eax
.notregAcc
	test eax, 0x8000
	jz .notTaskSwitch
	push task
	call display
	pop eax
.notTaskSwitch
	;test for each breakpoint being hit
	test eax, 1
	jz .not1
	push breakpoint
	call display
	pop eax
.not1
	test eax, 2
	jz .not2
	push breakpoint
	call display
	pop eax
.not2
	test eax, 4
	jz .not3
	push breakpoint
	call display
	pop eax
.not3
	test eax, 8
	jz .not4
	push breakpoint
	call display
	pop eax
.not4
	push breakpoint
	call display
	pop eax
	mov eax, [number]
	push eax
	call PrintNumber
	pop eax
	and eax, 0xFFFF1FF0
	mov dr6, eax	;clear all flags in the register for the next interrupt
	mov eax, [esp + 4]
	push eax
	call PrintNumber
	pop eax
	push newline
	call display
	pop eax
	pop eax
	jmp $
	iret

Two db 'NMI Interrupt', 10, 0
isr2:
;interrupt, no error code
;hmm what to do? i dont know...
	mov ax, 0x10
	mov ds, ax			;data segment (we can use variables with their name now)
	push Two
	call display		;print string (C++ function)
	pop eax
	jmp $
Three db 'Breakpoint', 10, 0
isr3:
;trap, no error code
	pusha
	mov ax, 0x10
	mov ds, ax			;data segment (we can use variables with their name now)
	push Three
	call display
	pop eax
	push newline
	call display
	pop eax
	mov eax, [esp + 0]
	push eax
	call PrintNumber		;print string (C++ function)
	pop eax
	push newline
	call display
	pop eax
	mov eax, [esp + 4]
	push eax
	call PrintNumber
	pop eax
	push newline
	call display
	pop eax
	popa
	iret
Four db 'Overflow', 10, 0
isr4:
;trap, no error code
	mov ax, 0x10
	mov ds, ax			;data segment (we can use variables with their name now)
	push Four
	call display		;print string (C++ function)
	pop eax
	jmp $
Five db 'Bounds range exceeded', 10, 0
isr5:
;fault, no error code
	mov ax, 0x10
	mov ds, ax			;data segment (we can use variables with their name now)
	push Five
	call display		;print string (C++ function)
	pop eax
	jmp $
Six db 'Invalid opcode', 10, 0
isr6:
;fault, no error code
	mov ax, 0x10
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
	mov ax, 0x10
	mov ds, ax			;data segment (we can use variables with their name now)
	push Seven
	call display		;print string (C++ function)
	pop eax
	jmp $
Eight db 'Double - fault', 10, 0
isr8:
;abort, error code does exist (it is zero)
;there is no return from here, the program must be closed, and data logged
	mov ax, 0x10
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
	mov ax, 0x10
	mov ds, ax			;data segment (we can use variables with their name now)
	push Nine
	call display		;print string (C++ function)
	pop eax
	jmp $
Ten db 'Invalid TSS exception', 10, 0
isr10:
;fault, error code present
;must use a task gate, to preserve stability
	mov ax, 0x10
	mov ds, ax			;data segment (we can use variables with their name now)
	push Ten
	call display		;print string (C++ function)
	pop eax
	jmp $
Eleven db 'Segment not present', 10, 0
isr11:
;fault, error code present
	mov ax, 0x10
	mov ds, ax			;data segment (we can use variables with their name now)
	push Eleven
	call display	;print string (C++ function)
	pop eax
	jmp $
Twelve db 'Stack fault exception', 10, 0
isr12:
;fault, error code present
	mov ax, 0x10
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
	mov ax, 0x10
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
	mov ax, 0x10
	mov ds, ax			;data segment (we can use variables with their name now)
	push Fifteen
	call display		;print string (C++ function)
	pop eax
	jmp $
Sixteen db 'x87 FPU Floating-Point Error', 10, 0
isr16:
;fault, no error code
	mov ax, 0x10
	mov ds, ax			;data segment (we can use variables with their name now)
	push Sixteen
	call display		;print string (C++ function)
	pop eax
	jmp $
Seventeen db 'Alignment Check Exception', 10, 0
isr17:
;fault, error code present
	mov ax, 0x10
	mov ds, ax			;data segment (we can use variables with their name now)
	push Seventeen
	call display		;print string (C++ function)
	pop eax
	jmp $
Eighteen db 'Machine-Check Exception', 10, 0
isr18:
;abort, no error code
	mov ax, 0x10
	mov ds, ax			;data segment (we can use variables with their name now)
	push Eighteen
	call display		;print string (C++ function)
	pop eax
	jmp $
Nineteen db 'SIMD Floating-Point Exception', 10, 0
isr19:
;fault, no error code
	mov ax, 0x10
	mov ds, ax			;data segment (we can use variables with their name now)
	push Nineteen
	call display		;print string (C++ function)
	pop eax
	jmp $

SECTION .bss
    resb 8192               ;8 KB for the stack, doesn't need to be initialized
stack:
