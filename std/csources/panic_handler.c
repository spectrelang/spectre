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

#ifdef __linux__
static int sx_print_frame(int idx, const char *sym_str)
{
    const char *paren = strchr(sym_str, '(');
    if (!paren) goto raw;
    const char *plus = strchr(paren, '+');
    if (!plus) goto raw;
    const char *addr_end = strchr(plus, ')');
    if (!addr_end) goto raw;

    {
        char exe[512];
        size_t exe_len = (size_t)(paren - sym_str);
        if (exe_len == 0 || exe_len >= sizeof(exe)) goto raw;
        memcpy(exe, sym_str, exe_len);
        exe[exe_len] = '\0';

        char addr[32];
        size_t addr_len = (size_t)(addr_end - (plus + 1));
        if (addr_len == 0 || addr_len >= sizeof(addr)) goto raw;
        memcpy(addr, plus + 1, addr_len);
        addr[addr_len] = '\0';

        char cmd[2048];
        snprintf(cmd, sizeof(cmd), "addr2line -e %s -f -p %s 2>/dev/null", exe, addr);
        FILE *fp = popen(cmd, "r");
        if (!fp) goto raw;
        char resolved[512] = "";
        if (fgets(resolved, sizeof(resolved), fp)) {
            size_t n = strlen(resolved);
            if (n > 0 && resolved[n-1] == '\n') resolved[n-1] = '\0';
        }
        pclose(fp);

        if (strstr(resolved, "panic_handler.c")) return 0;

        fprintf(stderr, "  #%-2d %s\n", idx, sym_str);
        if (resolved[0] && resolved[0] != '?')
            fprintf(stderr, "       at %s\n", resolved);
        return 1;
    }

raw:
    fprintf(stderr, "  #%-2d %s\n", idx, sym_str);
    return 1;
}
#endif

static const char *sx_signal_name(int sig)
{
    switch (sig) {
        case SIGSEGV: return "segmentation fault (sigsegv)";
        case SIGABRT: return "abort (sigabrt)";
        case SIGBUS:  return "bus error (sigbus)";
        case SIGFPE:  return "floating point exception (sigfpe)";
        case SIGILL:  return "illegal instruction (sigill)";
        default:      return "fatal signal";
    }
}

static void sx_signal_handler(int sig)
{
    void  *frames[64];
    int    nframes = backtrace(frames, 64);
    char **syms    = backtrace_symbols(frames, nframes);

    fprintf(stderr, "spectre: panic - %s\n", sx_signal_name(sig));
    fprintf(stderr, "trace:\n");

    if (syms) {
#ifdef __linux__
        int printed = 0;
        for (int i = 0; i < nframes; i++)
            printed += sx_print_frame(printed, syms[i]);
#else
        for (int i = 0; i < nframes; i++)
            fprintf(stderr, "  #%-2d %s\n", i, syms[i]);
#endif
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

    const char *desc = "unhandled exception";
    switch (info->ExceptionRecord->ExceptionCode) {
        case EXCEPTION_ACCESS_VIOLATION:
            desc = "access violation"; break;
        case EXCEPTION_STACK_OVERFLOW:
            desc = "stack overflow";                        break;
        case EXCEPTION_ILLEGAL_INSTRUCTION:
            desc = "illegal instruction (sigill)";          break;
        case EXCEPTION_INT_DIVIDE_BY_ZERO:
            desc = "integer divide by zero (sigfpe)";       break;
        case EXCEPTION_FLT_DIVIDE_BY_ZERO:
            desc = "float divide by zero (sigfpe)";         break;
    }

    fprintf(stderr, "\nspectre: panic - %s\n", desc);
    fprintf(stderr, "trace:\n");

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
