/*
 * Spectre freestanding entry point for linux.x86_64.
 * The initial stack is: argc, argv[0..argc-1], NULL, envp..., NULL, auxv...
 */
    .text
    .globl _start
    .type _start, @function
_start:
    xorq %rbp, %rbp
    movq (%rsp), %r12          /* argc */
    leaq 8(%rsp), %r13         /* argv */
    leaq 16(%rsp,%r12,8), %r14 /* envp */

    movq %r12, %rdi
    movq %r13, %rsi
    call __sx_set_argc_argv
    movq %r14, %rdi
    call __sx_set_envp

    movq %r12, %rdi
    movq %r13, %rsi
    andq $-16, %rsp
    call main

    movl %eax, %edi
    movl $231, %eax            /* exit_group */
    syscall
    hlt
    .size _start, .-_start

/*
 * long __sx_clone_thread(long flags, void *stack_top, void *entry, void *arg)
 * Enters a new thread using the clone(2) syscall.  The child starts on the
 * supplied stack and calls entry(arg); the parent returns the child tid.
 */
    .globl __sx_clone_thread
    .type __sx_clone_thread, @function
__sx_clone_thread:
    pushq %r12
    movq %rdx, %r9             /* entry */
    movq %rcx, %r12            /* arg */
    xorq %rdx, %rdx            /* ptid */
    xorq %r10, %r10            /* ctid */
    xorq %r8, %r8              /* TLS, unused */
    movq $56, %rax             /* clone */
    syscall
    testq %rax, %rax
    jnz 2f
    /* Child. */
    movq %r12, %rdi
    call *%r9
    movl %eax, %edi
    movl $60, %eax             /* exit */
    syscall
    hlt
2:
    popq %r12
    ret
    .size __sx_clone_thread, .-__sx_clone_thread

    .section .note.GNU-stack,"",@progbits
