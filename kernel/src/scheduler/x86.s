.section .text
.global thread_save
.global thread_restore
.global thread_wrapper1
.code64
thread_save:
    mov [rdi], rsp
    mov [rdi+8], rbx
    ret

thread_restore:
    mov rsp, [rdi]
    mov rbx, [rdi+8]
    ret

thread_wrapper1:
    pop rbx
    pop r12
    pop r13
    pop r14
    pop r15
    pop rbp
    ret
