/*
 * Hosted Windows compatibility shim.
 *
 * The hosted Windows path links the MSVC UCRT and Win32 SDK.  A handful of
 * POSIX names used by std/runtime are not provided by that environment; this
 * file supplies small Win32/UCRT-backed equivalents so the same runtime API
 * remains available in hosted mode.
 */
#if defined(_WIN32)

#include <winsock2.h>
#include <windows.h>
#include <direct.h>
#include <io.h>
#include <process.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

int _fltused = 0;

int rmdir(const char *path) { return _rmdir(path); }
int access(const char *path, int mode) { (void)mode; return _access(path, 0); }
int chdir(const char *path) { return _chdir(path); }
int unlink(const char *path) { return _unlink(path); }
int isatty(int fd) { return _isatty(fd); }
int fileno(FILE *f) { return _fileno(f); }
FILE *fdopen(int fd, const char *mode) { return _fdopen(fd, mode); }
char *strdup(const char *s) { return _strdup(s); }
char *strtok_r(char *s, const char *delim, char **save) { return strtok_s(s, delim, save); }
FILE *popen(const char *cmd, const char *mode) { return _popen(cmd, mode); }
int pclose(FILE *f) { return _pclose(f); }

int stat(const char *path, void *buf) { (void)path; (void)buf; return -1; }
int lstat(const char *path, void *buf) { (void)path; (void)buf; return -1; }
int fstat(int fd, void *buf) { (void)fd; (void)buf; return -1; }
int readlink(const char *path, char *buf, size_t n) { (void)path; (void)buf; (void)n; return -1; }

int uname(void *u) {
    (void)u;
    return -1;
}

char *realpath(const char *path, char *out) {
    if (out) return _fullpath(out, path, 4096);
    return (char *)path;
}

int gettimeofday(void *tv, void *tz) {
    (void)tz;
    FILETIME ft;
    GetSystemTimeAsFileTime(&ft);
    ULARGE_INTEGER u;
    u.LowPart = ft.dwLowDateTime;
    u.HighPart = ft.dwHighDateTime;
    unsigned long long us = (u.QuadPart - 116444736000000000ULL) / 10ULL;
    if (tv) {
        unsigned long long *p = (unsigned long long *)tv;
        p[0] = us / 1000000ULL;
        p[1] = us % 1000000ULL;
    }
    return 0;
}
int usleep(unsigned usec) { Sleep(usec / 1000u); return 0; }
int sleep(unsigned seconds) { Sleep(seconds * 1000u); return 0; }

HRESULT __stdcall BCryptGenRandom(void *, unsigned char *, unsigned long, unsigned long);

int getrandom(void *buf, size_t len, unsigned flags) {
    (void)flags;
    if (BCryptGenRandom(NULL, (unsigned char *)buf, (unsigned long)len, 2) == 0) {
        return (int)len;
    }
    return -1;
}
void arc4random_buf(void *buf, size_t len) { (void)getrandom(buf, len, 0); }


/* Directory iteration backed by FindFirstFileA. */
typedef struct hosted_dir {
    HANDLE h;
    WIN32_FIND_DATAA data;
    int first;
    struct {
        int d_ino;
        unsigned short d_reclen;
        unsigned short d_namlen;
        char d_name[260];
    } ent;
} hosted_dir;

void *opendir(const char *path) {
    char pattern[1024];
    size_t n = strlen(path);
    if (n + 3 >= sizeof(pattern)) return NULL;
    strcpy(pattern, path);
    if (n > 0 && pattern[n - 1] != '\\' && pattern[n - 1] != '/') {
        pattern[n] = '\\';
        pattern[n + 1] = '*';
        pattern[n + 2] = 0;
    } else {
        pattern[n] = '*';
        pattern[n + 1] = 0;
    }
    hosted_dir *d = (hosted_dir *)calloc(1, sizeof(hosted_dir));
    if (!d) return NULL;
    d->h = FindFirstFileA(pattern, &d->data);
    if (d->h == INVALID_HANDLE_VALUE) { free(d); return NULL; }
    d->first = 1;
    return d;
}
void *readdir(void *dir) {
    hosted_dir *d = (hosted_dir *)dir;
    if (!d) return NULL;
    if (!d->first) {
        if (!FindNextFileA(d->h, &d->data)) return NULL;
    }
    d->first = 0;
    d->ent.d_ino = 0;
    d->ent.d_reclen = (unsigned short)sizeof(d->ent);
    d->ent.d_namlen = (unsigned short)strlen(d->data.cFileName);
    strncpy(d->ent.d_name, d->data.cFileName, 259);
    d->ent.d_name[259] = 0;
    return &d->ent;
}
int closedir(void *dir) {
    hosted_dir *d = (hosted_dir *)dir;
    if (!d) return -1;
    FindClose(d->h);
    free(d);
    return 0;
}

/* These are only used by std/runtime's POSIX thread wrappers.  std/thread on
 * Windows uses CreateThread directly. */
int pthread_create(void *thread, void *attr, void *entry, void *arg) {
    (void)thread; (void)attr; (void)entry; (void)arg; return -1;
}
int pthread_join(void *thread, void *retval) { (void)thread; (void)retval; return -1; }
int pthread_detach(void *thread) { (void)thread; return -1; }
unsigned long long pthread_self(void) { return (unsigned long long)GetCurrentThreadId(); }
int sched_yield(void) { return SwitchToThread() ? 0 : -1; }
int pthread_mutex_init(void *m, void *attr) { (void)m; (void)attr; return 0; }
int pthread_mutex_destroy(void *m) { (void)m; return 0; }
int pthread_mutex_lock(void *m) { (void)m; return 0; }
int pthread_mutex_unlock(void *m) { (void)m; return 0; }

int poll(void *fds, unsigned long n, int timeout) { (void)fds; (void)n; (void)timeout; return -1; }

#endif
