/*
 * Spectre freestanding support for windows.aarch64.
 * __chkstk probes the stack for large frames; x15 carries the byte count.
 */
    .text
    .globl __chkstk
__chkstk:
    mov x16, sp
1:
    sub x16, x16, #4096
    ldr xzr, [x16]
    subs x15, x15, #4096
    b.hi 1b
    ret
