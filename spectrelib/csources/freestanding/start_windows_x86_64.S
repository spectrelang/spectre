/*
 * Spectre freestanding support routines for windows.x86_64.
 * The Windows ABI uses __chkstk before frames larger than one page.
 */
    .text
    .globl __chkstk
    .def __chkstk; .scl 2; .type 32; .endef
__chkstk:
    pushq %rcx
    pushq %rax
    cmpq $0x1000, %rax
    leaq 8(%rsp), %rcx
    jb 2f
1:
    subq $0x1000, %rcx
    testq %rcx, (%rcx)
    subq $0x1000, %rax
    cmpq $0x1000, %rax
    ja 1b
2:
    popq %rax
    popq %rcx
    ret
