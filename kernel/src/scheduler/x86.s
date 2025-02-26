.section .text
.global thread_save
.global thread_restore
.code64
thread_save:
    mov [rdi+0], rbp
    mov [rdi+8], rax
    mov [rdi+16], rbx
    mov [rdi+24], rcx
    mov [rdi+32], rdx
    mov [rdi+40], rsi
    mov [rdi+48], rdi
    mov [rdi+56], r8
    mov [rdi+64], r9
    mov [rdi+72], r10
    mov [rdi+80], r11
    mov [rdi+88], r12
    mov [rdi+96], r13
    mov [rdi+104], r14
    mov [rdi+112], r15
    pushfq
    pop [rdi+120]
    mov [rdi+128], rsp
    ret

thread_restore:
    mov rbp, [rdi+0]
    mov rax, [rdi+8]
    mov rbx, [rdi+16]
    mov rcx, [rdi+24]
    mov rdx, [rdi+32]
    mov rsi, [rdi+40]
    mov r8, [rdi+56]
    mov r9, [rdi+64]
    mov r10, [rdi+72]
    mov r11, [rdi+80]
    mov r12, [rdi+88]
    mov r13, [rdi+96]
    mov r14, [rdi+104]
    mov r15, [rdi+112]
    push [rdi+120]
    popfq
    mov rsp, [rdi+128]
    mov rdi, [rdi+48]
    
    
    
    ret
    