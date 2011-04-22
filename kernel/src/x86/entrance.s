#this is where execution starts
#this will be located at 0x100000 (1MB)
#[BITS 32]
#[extern display]               ;void display(char *chr)
#[extern PrintNumber]   ;void PrintNumber(unsigned long)
#[extern main]                  ;int main(
#[extern memcopy]
#[extern get_sys_tasks]
#[extern main]  ;this is in our C++ code
#[extern _main] ;this is in our C support code
#[extern _atexit] ;this is in our C support code
.globl timer
.globl syscall1

.globl start
start: 
        movl $stack, %esp  # This points the stack to our new stack area
        jmp skip

Error:
	.string "ERROR: The system was not booted with a multiboot compliant loader"

skip: 
#ebx contains the address of an important structure, multiboot_info
        cmpl $0x2BADB002,%eax
        je skip.hooray
        push Error
        call display
        popl %eax
here20:
        jmp here20
skip.hooray: 
        cli             #interrupt flag should already be cleared, but i'll disable them as an extra precaution
        movb $0xFF,%al  #disable PIC interrupts
        outb %al, $0x21 #disable PIC interrupts
        #mov [bootInfo], ebx    
        #this will be needed if the ebx register is ever touched
		xorl %eax, %eax
        movw $0x10,%ax
        movw %ax,%ds                    #set the segment registers
        movl %eax,%ss                   #and stack
        xorl %eax,%eax
        movl %eax,%es
        movl %eax,%fs
        movl %eax,%gs
        #refresh CS with a far jump
        ljmp $0x08, $flush2
flush2: 

#       [extern setupIdt]
        call setupIdt           #return value is stored in eax
        lidt (%eax)              #load the IDT

#       [extern kernel_end]
#       [extern __cxa_finalize]
        movl $kernel_end, %eax
        pushl %eax      #kernel size is the second argument
        movl %ebx,%eax
        #mov eax, [bootInfo]
        #use this instead if ebx is changed
        #call _main
        call detectCpu
        pushl %eax      #pointer is the first argument
        call main
        popl %eax
        pushl $0
        call __cxa_finalize
        popl %eax
        #call _atexit
END: 
        hlt
        jmp END

bootInfo:
	.long 0
#this is only needed if ebx is modified

.globl inportb
inportb: 
        movl 4(%esp),%edx
        xorl %eax,%eax
        inb %dx,%al
        ret

.globl inportw
inportw: 
        movl 4(%esp),%edx
        xorl %eax,%eax
        inw %dx,%ax
        ret

.globl outportb
outportb: 
        pushl %edx
        movl 8(%esp),%eax
        movl 12(%esp),%edx
        outb %al,%dx
        popl %edx
        ret

.globl outportw
outportw: 
        pushl %edx
        movl 8(%esp),%eax
        movl 12(%esp),%edx
        outw %ax,%dx
        popl %edx
        ret

.globl getEIP
getEIP: 
        movl (%esp),%eax
        ret



#[extern enter_spinlock]
#[extern leave_spinlock]
.globl test_and_set

test_and_set: 
        movl 4(%esp),%eax       #eax = new_value
        movl 8(%esp),%edx       #edx = lock_pointer
        lock xchgl %eax,(%edx)                  #swap *lock_pointer and new_value
        cmpl %eax,(%edx)
        je test_and_set #only return of the values are different
        ret                                                             #return the old value of *lock_pointer

#most interruptable
SL_BLANK:
	.word 0
			.word 0
SL_MEM_MNG:
	.long 1
SL_IRQ1:
	.long 2
SL_MESSAGE:
	.long 3
#least interruptable
#can go down the table, not up

newline:
	.string "\n"

.globl getCR3
getCR3: 
        movl %cr3, %eax
        ret

.globl invlpg_asm
invlpg_asm: 
        movl 4(%esp),%eax
        cmpb $1,processor_type
        jl invlpg_asm.skip #skip the opcode if the processor is a 386
        invlpg (%eax)
invlpg_asm.skip: 
        ret

processor_type:
	.byte 0

detectCpu:       #this should detect things that are detectable about the cpu
        movb $1,processor_type
        wbinvd  #this will determine if the processor is 486 or higher
                        #if it is not 486 or higher, then it is a 386
        cmpb $0,processor_type
        je detectCpu.done
        #do some more cpu testing
detectCpu.done: 
        ret
        #later  

.globl EnablePaging
EnablePaging: 
        movl 4(%esp),%eax     #get the base of the page directory (right above the kernel)
        movl %eax, %cr3         #time to set the paging bit
        movl %cr0, %eax         #also set the cache disable bit
        orl $0xE0000000,%eax
        movl %eax, %cr0
EnablePaging.keepgoing: 
        movl %cr0, %eax         #time to make sure it worked
        andl $0xE0000000,%eax
        cmpl $0xE0000000,%eax
        jne EnablePaging.keepgoing
        ret


.globl irqM0
.globl irqM1
.globl irqM2
.globl irqM3
.globl irqM4
.globl irqM5
.globl irqM6
.globl irqM7
.globl irqM8
.globl irqM9
.globl irqM10
.globl irqM11
.globl irqM12
.globl irqM13
.globl irqM14
.globl irqM15
.globl isr0
.globl isr1
.globl isr2
.globl isr3
.globl isr4
.globl isr5
.globl isr6
.globl isr7
.globl isr8
.globl isr9
.globl isr10
.globl isr11
.globl isr12
.globl isr13
.globl isr14
.globl isr15
.globl isr16
.globl isr17
.globl isr18
.globl isr19

.globl Delay
Delay:   #delays for some number of irq0 firings (each of which are about 1 millisecond)
        pushl %eax
        pushl %ebx
        movl timer,%eax
        addl 12(%esp),%eax              #eax = delay + time
.wait: 
		sti
		hlt
        movl timer,%ebx
        cmpl %ebx,%eax
        jg .wait
        popl %ebx
        popl %eax
        ret

.globl getDelayTime
getDelayTime:	#retrieves the delay time so that the DelayUntil function can be properly called
		movl timer, %eax
		ret

.globl DelayUntil
DelayUntil:	#delays until the timer reaches a certain point
		        pushl %eax
        pushl %ebx
        movl 12(%esp),%eax              #eax = delay + time
.waitUntil: 
		sti
		hlt
        movl timer,%ebx
        cmpl %ebx,%eax
        jg .waitUntil
        popl %ebx
        popl %eax
        ret


test_key:
	.byte 13
	.byte 0
PauseKey:
	.byte 0
.globl WaitKey
WaitKey: 
        pushw %ax
        movb $0,%al
        movb %al,PauseKey
WaitKey.wait: 
        cmpb PauseKey,%al
        je WaitKey.wait
        popw %ax
        ret


#the IRQ handler for the keyboard will convert scancodes and then place the code into a buffer
#the buffer will have a start and length
#codes will be pulled from the bottom of the buffer


#0xE0 0x2A 0xE0, 0x53
irqM1: 
        push %ax
        movb $0x20,%al
        outb %al, $0x20
		pop %ax
        iret


IRQM2:
	.string "IRQ2\n"
irqM2: 
        pushl %eax
        #manual EOI before the interrupt has ended
		push IRQM2
        call display
        popl %eax
        movb $0x20,%al
        outb %al, $0x20
        popl %eax
        iret

IRQM3:
	.string "IRQ3\n"
irqM3: 
        pushl %eax
        #manual EOI before the interrupt has ended
        push IRQM3
        call display
        popl %eax
        movb $0x20,%al
        outb %al, $0x20
        popl %eax
        iret

IRQM4:
	.string "IRQ4\n"
irqM4: 
        pushl %eax
        #manual EOI before the interrupt has ended
        push IRQM4
        call display
        popl %eax
        movb $0x20,%al
        outb %al, $0x20
        popl %eax
        iret

IRQM5:
	.string "IRQ5\n"
irqM5: 
        pushl %eax
        #manual EOI before the interrupt has ended
        push IRQM5
        call display
        popl %eax
        movb $0x20,%al
        outb %al, $0x20
        popl %eax
        iret

FloppyTimeout:
	.string "Timeout waiting for floppy drive\n"

.globl WaitFloppyInt
WaitFloppyInt: 
        pushl %ebx
        movl BytesDone,%eax
        movl timer,%ebx
        addl $0x200,%ebx
WaitFloppyInt.notThereYet: 
        cmpl timer,%ebx
        jl WaitFloppyInt.error
        cmpl BytesDone,%eax
        je WaitFloppyInt.notThereYet
        popl %ebx
        movl $0,%eax    #indicate success
        ret
WaitFloppyInt.error: 
        #push FloppyTimeout
        #call display
        #pop eax
        popl %ebx
        movl $0xFFFFFFFF,%eax   #-1 indicates failure
        ret

BytesDone:
	.long 0
IRQM6:
	.string "FDC has fired an interrupt!\n"
irqM6: 
#this is IRQ 6 from the master PIC
        pushw %ax
        incl BytesDone
#       push IRQM6
#       call display
#       pop eax
        #now return to wherever execution was before this interrupt
        movb $0x20,%al
        outb %al, $0x20
        popw %ax
        iret

IRQM7:
	.string "IRQ7\n"
irqM7: 
        pushl %eax
        #manual EOI before the interrupt has ended
        push IRQM7
        call display
        popl %eax
        movb $0x20,%al
        outb %al, $0x20
        popl %eax
        iret

IRQM8:
	.string "IRQ8\n"
irqM8: 
        pushl %eax
        #manual EOI before the interrupt has ended required for both master and slave PIC       
        push IRQM8
        call display
        popl %eax
        movb $0x20,%al
        outb %al, $0x20
        outb %al, $0xA0
        popl %eax
        iret

IRQM9:
	.string "IRQ9\n"
irqM9: 
        pushl %eax
        #manual EOI before the interrupt has ended required for both master and slave PIC       
        push IRQM9
        call display
        popl %eax
        movb $0x20,%al
        outb %al, $0x20
        outb %al, $0xA0
        popl %eax
        iret

IRQM10:
	.string "IRQ10\n"
irqM10: 
        pushl %eax
        #manual EOI before the interrupt has ended required for both master and slave PIC       
        push IRQM10
        call display
        popl %eax
        movb $0x20,%al
        outb %al, $0x20
        outb %al, $0xA0
        popl %eax
        iret

IRQM11:
	.string "IRQ11\n"
irqM11: 
        pushl %eax
        #manual EOI before the interrupt has ended required for both master and slave PIC       
        push IRQM11
        call display
        popl %eax
        movb $0x20,%al
        outb %al, $0x20
        outb %al, $0xA0
        popl %eax
        iret

IRQM12:
	.string "IRQ12\n"
irqM12: 
        pushl %eax
        #manual EOI before the interrupt has ended required for both master and slave PIC       
        push IRQM12
        call display
        popl %eax
        movb $0x20,%al
        outb %al, $0x20
        outb %al, $0xA0
        popl %eax
        iret

IRQM13:
	.string "IRQ13\n"
irqM13: 
        pushl %eax
        #manual EOI before the interrupt has ended required for both master and slave PIC       
        push IRQM13
        call display
        popl %eax
        movb $0x20,%al
        outb %al, $0x20
        outb %al, $0xA0
        popl %eax
        iret

.globl HD_INTS
.align 4
HD_INTS:
	.long 0   
IRQM14:
	.string "IRQ14\n"
irqM14: 
        pushl %eax
        incl HD_INTS

        push IRQM14
        call display
        popl %eax
        #manual EOI before the interrupt has ended required for both master and slave PIC
        movb $0x20,%al
        outb %al, $0x20
        outb %al, $0xA0
        popl %eax
        iret

IRQM15:
	.string "IRQ15\n"
irqM15: 
        pushl %eax

        push IRQM15
        call display
        popl %eax
        #manual EOI before the interrupt has ended required for both master and slave PIC
        movb $0x20,%al
        outb %al, $0x20
        outb %al, $0xA0
        popl %eax
        iret




Code:
	.long 0
Zero:
	.string "Divide by zero error!\n"
isr0: 
#fault, no error code
#display message, stop, because returning brings it back
        movw $0x10,%ax
        movw %ax,%ds                    #data segment (we can use variables with their name now)
        push Zero
        call display            #print string (C++ function)
        popl %eax
here19:
        jmp here19

One:
	.string "Debug exception, "
isr1: 
#trap or fault, no error code
#examine DR6 and other debug registers to determine type
        pushl %eax
        push One
        call display
        popl %eax
here18:
        jmp here18
        iret

Two:
	.string "Non-Maskable Interrupt\n"
isr2: 
#interrupt, no error code
#hmm what to do? i dont know...
        movw $0x10,%ax
        movw %ax,%ds                    #data segment (we can use variables with their name now)
        push Two
        call display            #print string (C++ function)
        popl %eax
here17:
        jmp here17
Three:
	.string "Breakpoint\n"
isr3: 
#trap, no error code
        pusha
        movw $0x10,%ax
        movw %ax,%ds                    #data segment (we can use variables with their name now)
        push Three
        call display
        popl %eax
        push newline
        call display
        popl %eax
        movl 0(%esp),%eax
        pushl %eax
        call PrintNumber                #print string (C++ function)
        popl %eax
        push newline
        call display
        popl %eax
        movl 4(%esp),%eax
        pushl %eax
        call PrintNumber
        popl %eax
        push newline
        call display
        popl %eax
        popa
        iret
Four:
	.string "Overflow\n"
isr4: 
#trap, no error code
        movw $0x10,%ax
        movw %ax,%ds                    #data segment (we can use variables with their name now)
        push Four
        call display            #print string (C++ function)
        popl %eax
here16:
        jmp here16
Five:
	.string "Bounds range exceeded\n"
isr5: 
#fault, no error code
        movw $0x10,%ax
        movw %ax,%ds                    #data segment (we can use variables with their name now)
        push Five
        call display            #print string (C++ function)
        popl %eax
here15:
        jmp here15

Six:
	.string "Invalid opcode\n"
backup:
	.long 0

isr6: 
#fault, no error code
        pushl %eax
        movw $0x10,%ax
        movw %ax,%ds                    #data segment (we can use variables with their name now)
        popl %eax
        movl %eax,backup
        push Six
        call display            #print string (C++ function)
        popl %eax
#our stack contains: EIP, CS
        call PrintNumber
        popl %ebx
        call PrintNumber
        popw %ax
        movw %ax,%es
        cmpw $0x090F, %es:(%ebx)             #this is the opcode for wbinvd
        je isr6.Wbinvd
        pushw %ax
        pushl %ebx
        jmp isr6.Other
isr6.Wbinvd: 
        addl $2,%ebx            #skip that instruction (it wont hurt anything)
        #this means a 386 or lower processor has been detected
        movb $0,processor_type
        pushw %ax       #add stuff to the stack for a proper iret
call PrintNumber
        pushl %ebx
call PrintNumber
        movl backup,%eax
        iret
isr6.Other: 
        #check for cpuid and other things
here14:
        jmp here14
Seven:
	.string "Device not available\n"

isr7: 
#fault, no error code
#this is confusing
        movw $0x10,%ax
        movw %ax,%ds                    #data segment (we can use variables with their name now)
        push Seven
        call display            #print string (C++ function)
        popl %eax
here13:
        jmp here13

Eight:
	.string "Double - fault\n"

isr8: 
#abort, error code does exist (it is zero)
#there is no return from here, the program must be closed, and data logged
        movw $0x10,%ax
        movw %ax,%ds                    #data segment (we can use variables with their name now)
        push Eight
        call display            #print string (C++ function)
        popl %eax
        cli                             #disable interrupts
        hlt                             #hang

Nine:
	.string "Coprocessor segment overrun\n"

isr9: 
#abort, no error code
#FPU must be restarted (so we won't return for now)
        movw $0x10,%ax
        movw %ax,%ds                    #data segment (we can use variables with their name now)
        push Nine
        call display            #print string (C++ function)
        popl %eax
here12:
        jmp here12

Ten:
	.string "Invalid TSS exception\n"

isr10: 
#fault, error code present
#must use a task gate, to preserve stability
        movw $0x10,%ax
        movw %ax,%ds                    #data segment (we can use variables with their name now)
        push Ten
        call display            #print string (C++ function)
        popl %eax
here11:
        jmp here11

Eleven:
	.string "Segment not present\n"

isr11: 
#fault, error code present
        movw $0x10,%ax
        movw %ax,%ds                    #data segment (we can use variables with their name now)
        push Eleven
        call display    #print string (C++ function)
        popl %eax
here10:
        jmp here10

Twelve:
	.string "Stack fault exception\n"

isr12: 
#fault, error code present
        movw $0x10,%ax
        movw %ax,%ds                    #data segment (we can use variables with their name now)
        push Twelve
        call display    #print string (C++ function)
        popl %eax
here9:
        jmp here9

Thir1:
	.string "*Error Code:"
Thir2:
	.string ",EIP:"
Thir3:
	.string ",CS:"
Thir4:
	.string ",EFLAGS:"
Thir5:
	.string ",ESP:"
Thir6:
	.string "SS:\n"
Thirteen:
	.string "General Protection Fault\n"

isr13: 
#fault, error code present
        movw $0x10,%ax
        movw %ax,%ds                    #data segment (we can use variables with their name now)
        push Thirteen
        call display    #print string (C++ function)
        popl %eax
#       push Thir1
#       call display
#       pop eax
#       call PrintNumber
#       pop eax
#       push Thir2
#       call display
#       pop eax
#       call PrintNumber
#       pop eax
#       push Thir3
#       call display
#       pop eax
#       call PrintNumber
#       pop eax
#       push Thir4
#       call display
#       pop eax
#       call PrintNumber
#       pop eax
#       push Thir5
#       call display
#       pop eax
#       call PrintNumber
#       pop eax
#       push Thir6
#       call display
#       pop eax
#       call PrintNumber
#       pop eax
here8:
        jmp here8

Fourteen:
	.string "Page Fault Exception\n"
_Fourteen:
	.string "A reserved bit has been set in the page directory!\n"
_2Fourteen:
	.string "A page level protection violation has occurred!\n"
_3Fourteen:
	 .string "A page that does not exist in RAM has been accessed\n"
Location:
	 .long 0
_EAX:
	 .long 0

isr14: 
#fault, special error code format same size though
#call through task gate, to allow page faulting during task switches
#first, place all regs 
        pushl %eax
        movw $0x10,%ax
        movw %ax,%ds                    #data segment (we can use variables with their name now)
        popl %eax
        #store eax in a variable, so we can allow for an error code
        movl %eax,_EAX
        #get the error code
        popl %eax
        movl %eax,Code
        movl $_EAX, %eax
        pushal                  #do a popad     before returning to code
        movl %cr2, %eax
        pushl %eax                      #save that address to the stack
        push Fourteen
        call display            #print string (C++ function)
        popl %eax
        #check that a reserved bit was not set in the page directory (thats bad)
        movl $Code, %eax
        andl $0b1000,%eax
        cmpl $0b1000,%eax
        jne .Yay
        push _Fourteen
        call display
        popl %eax
        #display cr2
.here7:
        jmp .here7
.Yay: 
        #CS|EIP|PUSHAD|CR2|
        movl $Code, %eax
        andl $1,%eax
        cmpl $1,%eax
        jne .Ok
        #page level protection violation
        push _2Fourteen
        call display
        popl %eax
.here6:
        jmp .here6
.Ok: 
        push _3Fourteen
        call display
        popl %eax
.here5:
        jmp .here5

Fifteen:
	.string "Intel reserved interrupt has been called - this is bad\n"

isr15: 
#fault, no error code
        movw $0x10,%ax
        movw %ax,%ds                    #data segment (we can use variables with their name now)
        push Fifteen
        call display            #print string (C++ function)
        popl %eax
here4:
        jmp here4

Sixteen:
	.string "x87 FPU Floating-Point Error\n"

isr16: 
#fault, no error code
        movw $0x10,%ax
        movw %ax,%ds                    #data segment (we can use variables with their name now)
        push Sixteen
        call display            #print string (C++ function)
        popl %eax
here2:
        jmp here2

Seventeen:
	.string "Alignment Check Exception\n"

isr17: 
#fault, error code present
        movw $0x10,%ax
        movw %ax,%ds                    #data segment (we can use variables with their name now)
        push Seventeen
        call display            #print string (C++ function)
        popl %eax
here3:
        jmp here3

Eighteen:
	.string "Machine-Check Exception\n"

isr18: 
#abort, no error code
        movw $0x10,%ax
        movw %ax,%ds                    #data segment (we can use variables with their name now)
        push Eighteen
        call display            #print string (C++ function)
        popl %eax
here1:
        jmp here1

Nineteen:
	.string "SIMD Floating-Point Exception\n"

isr19: 
#fault, no error code
        movw $0x10,%ax
        movw %ax,%ds                    #data segment (we can use variables with their name now)
        push Nineteen
        call display            #print string (C++ function)
        popl %eax
here0:
        jmp here0

.bss
    .lcomm buffer 8192
stack: 

