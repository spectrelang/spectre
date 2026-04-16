#if defined(_WIN32) || defined(_WIN64)
#  define SX_PLATFORM_WINDOWS 1
#else
#  define SX_PLATFORM_POSIX 1
#endif

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#ifdef SX_PLATFORM_POSIX

#include <signal.h>
#include <execinfo.h>
#include <unistd.h>

static const char *sx_signal_name(int sig)
{
    switch (sig) {
        case SIGSEGV: return "Segmentation Fault (SIGSEGV)";
        case SIGABRT: return "Abort (SIGABRT)";
        case SIGBUS:  return "Bus Error (SIGBUS)";
        case SIGFPE:  return "Floating Point Exception (SIGFPE)";
        case SIGILL:  return "Illegal Instruction (SIGILL)";
        default:      return "Fatal Signal";
    }
}

static void sx_signal_handler(int sig)
{
    void  *frames[64];
    int    nframes = backtrace(frames, 64);
    char **syms    = backtrace_symbols(frames, nframes);

    fprintf(stderr, "\nspectre: RuntimeError - %s\n", sx_signal_name(sig));
    fprintf(stderr, "Stack trace:\n");

    if (syms) {
        for (int i = 0; i < nframes; i++)
            fprintf(stderr, "  #%-2d %s\n", i, syms[i]);
        free(syms);
    } else {
        backtrace_symbols_fd(frames, nframes, STDERR_FILENO);
    }

    fflush(stderr);
    signal(sig, SIG_DFL);
    raise(sig);
}

void __sx_panic_init(void)
{
    struct sigaction sa;
    memset(&sa, 0, sizeof(sa));
    sa.sa_handler = sx_signal_handler;
    sigemptyset(&sa.sa_mask);
    sa.sa_flags = SA_RESETHAND;
    sigaction(SIGSEGV, &sa, NULL);
    sigaction(SIGABRT, &sa, NULL);
    sigaction(SIGBUS,  &sa, NULL);
    sigaction(SIGFPE,  &sa, NULL);
    sigaction(SIGILL,  &sa, NULL);
}

#endif

#ifdef SX_PLATFORM_WINDOWS

#include <windows.h>
#include <dbghelp.h>

static LONG WINAPI sx_exception_handler(EXCEPTION_POINTERS *info)
{
    HANDLE proc = GetCurrentProcess();
    SymInitialize(proc, NULL, TRUE);

    const char *desc = "Unhandled Exception";
    switch (info->ExceptionRecord->ExceptionCode) {
        case EXCEPTION_ACCESS_VIOLATION:
            desc = "Access Violation (SIGSEGV equivalent)"; break;
        case EXCEPTION_STACK_OVERFLOW:
            desc = "Stack Overflow";                        break;
        case EXCEPTION_ILLEGAL_INSTRUCTION:
            desc = "Illegal Instruction (SIGILL)";          break;
        case EXCEPTION_INT_DIVIDE_BY_ZERO:
            desc = "Integer Divide By Zero (SIGFPE)";       break;
        case EXCEPTION_FLT_DIVIDE_BY_ZERO:
            desc = "Float Divide By Zero (SIGFPE)";         break;
    }

    fprintf(stderr, "\nspectre: RuntimeError - %s\n", desc);
    fprintf(stderr, "Stack trace:\n");

    void        *stack[64];
    WORD         nframes = CaptureStackBackTrace(0, 64, stack, NULL);
    SYMBOL_INFO *sym     = (SYMBOL_INFO *)calloc(sizeof(SYMBOL_INFO) + 256, 1);

    if (sym) {
        sym->MaxNameLen   = 255;
        sym->SizeOfStruct = sizeof(SYMBOL_INFO);
        for (WORD i = 0; i < nframes; i++) {
            SymFromAddr(proc, (DWORD64)stack[i], 0, sym);
            fprintf(stderr, "  #%-2u %s (0x%016llx)\n",
                    (unsigned)i,
                    sym->Name,
                    (unsigned long long)sym->Address);
        }
        free(sym);
    }

    fflush(stderr);
    return EXCEPTION_EXECUTE_HANDLER;
}

void __sx_panic_init(void)
{
    SetUnhandledExceptionFilter(sx_exception_handler);
}

#endif
