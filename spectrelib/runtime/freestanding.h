/*
 * Generated freestanding C ABI header.
 *
 * The C99 backend emits this shape directly and intentionally avoids all
 * system headers.  Keep this file in sync with runtime.c.
 */
#ifndef SPECTRE_FREESTANDING_H
#define SPECTRE_FREESTANDING_H

typedef __SIZE_TYPE__ sx_size_t;
typedef __PTRDIFF_TYPE__ sx_ssize_t;
typedef __INTPTR_TYPE__ sx_intptr_t;
typedef __UINTPTR_TYPE__ sx_uintptr_t;
typedef __INT8_TYPE__ sx_i8;
typedef __UINT8_TYPE__ sx_u8;
typedef __INT16_TYPE__ sx_i16;
typedef __UINT16_TYPE__ sx_u16;
typedef __INT32_TYPE__ sx_i32;
typedef __UINT32_TYPE__ sx_u32;
typedef __INT64_TYPE__ sx_i64;
typedef __UINT64_TYPE__ sx_u64;

void *malloc(sx_size_t size);
void *calloc(sx_size_t count, sx_size_t size);
void *realloc(void *ptr, sx_size_t size);
void free(void *ptr);

void *memcpy(void *dst, const void *src, sx_size_t n);
void *memmove(void *dst, const void *src, sx_size_t n);
void *memset(void *dst, int value, sx_size_t n);
int memcmp(const void *a, const void *b, sx_size_t n);
sx_size_t strlen(const char *s);
int strcmp(const char *a, const char *b);
int strncmp(const char *a, const char *b, sx_size_t n);
char *strstr(const char *haystack, const char *needle);

int printf(const char *fmt, ...);
int snprintf(char *buf, sx_size_t cap, const char *fmt, ...);
int dprintf(int fd, const char *fmt, ...);

sx_ssize_t read(int fd, void *buf, sx_size_t n);
sx_ssize_t write(int fd, const void *buf, sx_size_t n);
int close(int fd);

void *__sx_alloc(sx_size_t n);
void *__sx_calloc(sx_size_t count, sx_size_t n);
void *__sx_realloc(void *p, sx_size_t n);
void __sx_free(void *p);
void *__sx_memcpy(void *dst, const void *src, sx_size_t n);
void *__sx_memset(void *dst, int v, sx_size_t n);
sx_size_t __sx_strlen(const char *s);
sx_ssize_t __sx_read(int fd, void *buf, sx_size_t n);
sx_ssize_t __sx_write(int fd, const void *buf, sx_size_t n);
int __sx_close(int fd);
void __sx_exit(int code);
sx_i64 __sx_clock(void);
void __sx_sleep(unsigned millis);
void __sx_random_fill(void *buf, sx_size_t n);

#endif
