.section .text
.global thread_save
.global thread_restore
.code64
thread_save:
    mov [rbx+0], rbp
    mov [rbx+8], rax
    mov [rbx+16], rbx
    mov [rbx+24], rcx
    mov [rbx+32], rdx
    mov [rbx+40], rsi
    mov [rbx+48], rdi
    mov [rbx+56], r8
    mov [rbx+64], r9
    mov [rbx+72], r10
    mov [rbx+80], r11
    mov [rbx+88], r12
    mov [rbx+96], r13
    mov [rbx+104], r14
    mov [rbx+112], r15
    pushfq
    pop [rbx+120]
    ret

thread_restore:
    mov rbp, [rbx+0]
    mov rax, [rbx+8]
    mov rcx, [rbx+24]
    mov rdx, [rbx+32]
    mov rsi, [rbx+40]
    mov rdi, [rbx+48]
    mov r8, [rbx+56]
    mov r9, [rbx+64]
    mov r10, [rbx+72]
    mov r11, [rbx+80]
    mov r12, [rbx+88]
    mov r13, [rbx+96]
    mov r14, [rbx+104]
    mov r15, [rbx+112]
    push [rbx+120]
    popfq
    mov rbx, [rbx+16]
    ret
    