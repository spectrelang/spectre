/*
 * Spectre freestanding entry point for linux.aarch64.
 */
    .text
    .globl _start
    .type _start, %function
_start:
    mov x19, sp                /* initial stack */
    ldr x20, [x19]             /* argc */
    add x21, x19, #8           /* argv */
    add x22, x21, x20, lsl #3  /* envp */

    mov x0, x20
    mov x1, x21
    bl __sx_set_argc_argv
    mov x0, x22
    bl __sx_set_envp

    mov x0, x20
    mov x1, x21
    and sp, x19, #-16
    mov x29, #0
    mov x30, #0
    bl main

    mov w19, w0
    mov x8, #94                /* exit_group */
    mov x0, x19
    svc #0
    brk #0
    .size _start, .-_start

/*
 * long __sx_clone_thread(long flags, void *stack_top, void *entry, void *arg)
 */
    .globl __sx_clone_thread
    .type __sx_clone_thread, %function
__sx_clone_thread:
    stp x19, x20, [sp, #-16]!
    mov x19, x2                /* entry */
    mov x20, x3                /* arg */
    mov x2, #0                 /* ptid */
    mov x3, #0                 /* tls */
    mov x4, #0                 /* ctid */
    mov x8, #220               /* clone */
    svc #0
    cbnz x0, 2f
    mov x0, x20
    blr x19
    mov x8, #93                /* exit */
    svc #0
2:
    ldp x19, x20, [sp], #16
    ret
    .size __sx_clone_thread, .-__sx_clone_thread

    .section .note.GNU-stack,"",%progbits
