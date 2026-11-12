/*
 * Spectre freestanding runtime.
 *
 * This file is compiled with -ffreestanding -nostdinc.  It must not include
 * any system header and must not call into the host C library.  Linux uses
 * direct syscalls; Windows uses documented kernel32/ntdll/bcrypt/ws2_32
 * entry points.
 *
 * SPDX-License-Identifier: GPL-3.0-only
 * (C) Navid Momtahen - 2026
 */

#if defined(_WIN32)
#define SX_WINDOWS 1
#else
#define SX_LINUX 1
#endif

typedef __SIZE_TYPE__ sx_size_t;
typedef __PTRDIFF_TYPE__ sx_ssize_t;
typedef __INTPTR_TYPE__ sx_intptr_t;
typedef __UINTPTR_TYPE__ sx_uintptr_t;
typedef __INT64_TYPE__ sx_i64;
typedef __UINT64_TYPE__ sx_u64;
typedef __INT32_TYPE__ sx_i32;
typedef __UINT32_TYPE__ sx_u32;
typedef __INT16_TYPE__ sx_i16;
typedef __UINT16_TYPE__ sx_u16;
typedef __INT8_TYPE__ sx_i8;
typedef __UINT8_TYPE__ sx_u8;

typedef __builtin_va_list sx_va_list;

#define SX_NULL ((void *)0)

sx_i64 __sx_sys_getrandom(void *buf, sx_size_t n);
sx_i64 __sx_sys_clock_us(void);
sx_i64 __sx_sys_nanosleep_us(sx_i64 us);

/* ------------------------------------------------------------------------- */
/* Shared memory / string helpers                                            */
/* ------------------------------------------------------------------------- */

void *memcpy(void *dst, const void *src, sx_size_t n) {
  unsigned char *d = (unsigned char *)dst;
  const unsigned char *s = (const unsigned char *)src;
  while (n > 0 && (((sx_size_t)d) & (sizeof(sx_size_t) - 1)) != 0) {
    *d++ = *s++;
    n--;
  }
  sx_size_t *dw = (sx_size_t *)d;
  const sx_size_t *sw = (const sx_size_t *)s;
  while (n >= sizeof(sx_size_t)) {
    *dw++ = *sw++;
    n -= sizeof(sx_size_t);
  }
  d = (unsigned char *)dw;
  s = (const unsigned char *)sw;
  while (n > 0) {
    *d++ = *s++;
    n--;
  }
  return dst;
}

void *memmove(void *dst, const void *src, sx_size_t n) {
  unsigned char *d = (unsigned char *)dst;
  const unsigned char *s = (const unsigned char *)src;
  if (d == s || n == 0) {
    return dst;
  }
  if (d < s || d >= s + n) {
    return memcpy(dst, src, n);
  }
  sx_size_t i = n;
  while (i > 0 && ((((sx_size_t)(d + i)) & (sizeof(sx_size_t) - 1)) != 0)) {
    i--;
    d[i] = s[i];
  }
  sx_size_t *dw = (sx_size_t *)(d + i);
  const sx_size_t *sw = (const sx_size_t *)(s + i);
  while (i >= sizeof(sx_size_t)) {
    dw--;
    sw--;
    *dw = *sw;
    i -= sizeof(sx_size_t);
  }
  while (i > 0) {
    i--;
    d[i] = s[i];
  }
  return dst;
}

void *memset(void *dst, int value, sx_size_t n) {
  unsigned char *d = (unsigned char *)dst;
  unsigned char v = (unsigned char)value;
  while (n > 0 && (((sx_size_t)d) & (sizeof(sx_size_t) - 1)) != 0) {
    *d++ = v;
    n--;
  }
  sx_size_t word = (sx_size_t)v;
  sx_size_t shift = 8;
  while (shift < sizeof(sx_size_t) * 8) {
    word |= ((sx_size_t)v) << shift;
    shift += 8;
  }
  sx_size_t *dw = (sx_size_t *)d;
  while (n >= sizeof(sx_size_t)) {
    *dw++ = word;
    n -= sizeof(sx_size_t);
  }
  d = (unsigned char *)dw;
  while (n > 0) {
    *d++ = v;
    n--;
  }
  return dst;
}

void *memchr(const void *s, int c, sx_size_t n) {
  const unsigned char *p = (const unsigned char *)s;
  const unsigned char needle = (unsigned char)c;
  while (n > 0 && (((sx_size_t)p) & (sizeof(sx_size_t) - 1)) != 0) {
    if (*p == needle) {
      return (void *)p;
    }
    p++;
    n--;
  }
  const sx_size_t ones = (sx_size_t)-1 / 255;
  const sx_size_t highs = ones * 128;
  const sx_size_t target = ones * needle;
  const sx_size_t *wp = (const sx_size_t *)p;
  while (n >= sizeof(sx_size_t)) {
    sx_size_t w = *wp ^ target;
    if (((w - ones) & ~w & highs) != 0) {
      const unsigned char *q = (const unsigned char *)wp;
      sx_size_t i = 0;
      while (i < sizeof(sx_size_t)) {
        if (q[i] == needle) {
          return (void *)(q + i);
        }
        i++;
      }
    }
    wp++;
    n -= sizeof(sx_size_t);
  }
  p = (const unsigned char *)wp;
  while (n > 0) {
    if (*p == needle) {
      return (void *)p;
    }
    p++;
    n--;
  }
  return 0;
}

int memcmp(const void *a, const void *b, sx_size_t n) {
  const unsigned char *x = (const unsigned char *)a;
  const unsigned char *y = (const unsigned char *)b;
  while (n >= sizeof(sx_size_t)) {
    sx_size_t xw = *(const sx_size_t *)x;
    sx_size_t yw = *(const sx_size_t *)y;
    if (xw != yw) {
      break;
    }
    x += sizeof(sx_size_t);
    y += sizeof(sx_size_t);
    n -= sizeof(sx_size_t);
  }
  while (n > 0) {
    if (*x != *y) {
      return (int)*x - (int)*y;
    }
    x++;
    y++;
    n--;
  }
  return 0;
}

sx_size_t strlen(const char *s) {
  if (!s) {
    return 0;
  }
  const unsigned char *p = (const unsigned char *)s;
  while ((((sx_size_t)p) & (sizeof(sx_size_t) - 1)) != 0) {
    if (*p == 0) {
      return (sx_size_t)(p - (const unsigned char *)s);
    }
    p++;
  }
  const sx_size_t *wp = (const sx_size_t *)p;
  const sx_size_t ones = (sx_size_t)-1 / 255;
  const sx_size_t highs = ones * 128;
  while (1) {
    sx_size_t w = *wp;
    if (((w - ones) & ~w & highs) != 0) {
      const unsigned char *q = (const unsigned char *)wp;
      sx_size_t i = 0;
      while (i < sizeof(sx_size_t)) {
        if (q[i] == 0) {
          return (sx_size_t)(q + i - (const unsigned char *)s);
        }
        i++;
      }
    }
    wp++;
  }
}

int strcmp(const char *a, const char *b) {
  if (!a) {
    a = "";
  }
  if (!b) {
    b = "";
  }
  while (*a && *a == *b) {
    a++;
    b++;
  }
  return (int)(unsigned char)*a - (int)(unsigned char)*b;
}

int strncmp(const char *a, const char *b, sx_size_t n) {
  sx_size_t i = 0;
  while (i < n) {
    unsigned char ca = a ? (unsigned char)a[i] : 0;
    unsigned char cb = b ? (unsigned char)b[i] : 0;
    if (ca != cb) {
      return (int)ca - (int)cb;
    }
    if (ca == 0) {
      return 0;
    }
    i++;
  }
  return 0;
}

char *strcpy(char *dst, const char *src) {
  char *d = dst;
  while (*src) {
    *d++ = *src++;
  }
  *d = 0;
  return dst;
}

char *strncpy(char *dst, const char *src, sx_size_t n) {
  sx_size_t i = 0;
  while (i < n && src[i]) {
    dst[i] = src[i];
    i++;
  }
  while (i < n) {
    dst[i] = 0;
    i++;
  }
  return dst;
}

char *strcat(char *dst, const char *src) {
  char *d = dst + strlen(dst);
  strcpy(d, src);
  return dst;
}

char *strchr(const char *s, int c) {
  if (!s) {
    return SX_NULL;
  }
  while (*s) {
    if (*s == (char)c) {
      return (char *)s;
    }
    s++;
  }
  return (c == 0) ? (char *)s : SX_NULL;
}

char *strrchr(const char *s, int c) {
  const char *last = SX_NULL;
  if (!s) {
    return SX_NULL;
  }
  while (*s) {
    if (*s == (char)c) {
      last = s;
    }
    s++;
  }
  if (c == 0) {
    return (char *)s;
  }
  return (char *)last;
}

char *strstr(const char *haystack, const char *needle) {
  if (!haystack || !needle) {
    return SX_NULL;
  }
  if (!*needle) {
    return (char *)haystack;
  }
  while (*haystack) {
    const char *h = haystack;
    const char *n = needle;
    while (*h && *n && *h == *n) {
      h++;
      n++;
    }
    if (!*n) {
      return (char *)haystack;
    }
    haystack++;
  }
  return SX_NULL;
}

int atoi(const char *s) {
  int sign = 1;
  int value = 0;
  if (!s) {
    return 0;
  }
  while (*s == ' ' || *s == '\t' || *s == '\n' || *s == '\r') {
    s++;
  }
  if (*s == '-') {
    sign = -1;
    s++;
  } else if (*s == '+') {
    s++;
  }
  while (*s >= '0' && *s <= '9') {
    value = value * 10 + (int)(*s - '0');
    s++;
  }
  return sign * value;
}

long strtol(const char *s, char **end, int base) {
  long value = 0;
  int sign = 1;
  int any = 0;
  if (!s) {
    if (end)
      *end = (char *)s;
    return 0;
  }
  while (*s == ' ' || *s == '\t' || *s == '\n' || *s == '\r')
    s++;
  if (*s == '-') {
    sign = -1;
    s++;
  } else if (*s == '+')
    s++;
  if ((base == 0 || base == 16) && s[0] == '0' &&
      (s[1] == 'x' || s[1] == 'X')) {
    base = 16;
    s += 2;
  } else if (base == 0 && s[0] == '0') {
    base = 8;
  } else if (base == 0) {
    base = 10;
  }
  for (;;) {
    int digit;
    char c = *s;
    if (c >= '0' && c <= '9')
      digit = c - '0';
    else if (c >= 'a' && c <= 'f')
      digit = c - 'a' + 10;
    else if (c >= 'A' && c <= 'F')
      digit = c - 'A' + 10;
    else
      break;
    if (digit >= base)
      break;
    value = value * (long)base + (long)digit;
    any = 1;
    s++;
  }
  if (end)
    *end = (char *)(any ? s : (s - 1));
  return sign * value;
}

int isdigit(int c) { return c >= '0' && c <= '9'; }
int isalpha(int c) { return (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z'); }
int isalnum(int c) { return isalpha(c) || isdigit(c); }
int isspace(int c) {
  return c == ' ' || c == '\t' || c == '\n' || c == '\r' || c == '\v' ||
         c == '\f';
}
int isupper(int c) { return c >= 'A' && c <= 'Z'; }
int islower(int c) { return c >= 'a' && c <= 'z'; }
int toupper(int c) { return islower(c) ? c - 32 : c; }
int tolower(int c) { return isupper(c) ? c + 32 : c; }

/* ------------------------------------------------------------------------- */
/* Formatting                                                               */
/* ------------------------------------------------------------------------- */

typedef struct {
  char *buf;
  sx_size_t cap;
  sx_size_t len;
  int overflow;
} sx_out;

static void sx_putc(sx_out *o, char c) {
  if (o->len + 1 < o->cap) {
    o->buf[o->len] = c;
  } else {
    o->overflow = 1;
  }
  o->len++;
}

static void sx_puts_out(sx_out *o, const char *s) {
  if (!s)
    s = "(null)";
  while (*s) {
    sx_putc(o, *s++);
  }
}

static void sx_put_repeat(sx_out *o, char c, int count) {
  while (count-- > 0)
    sx_putc(o, c);
}

static void sx_utoa_base(sx_u64 value, unsigned base, int upper, char *out) {
  static const char lower[] = "0123456789abcdef";
  static const char upper_chars[] = "0123456789ABCDEF";
  const char *digits = upper ? upper_chars : lower;
  char tmp[66];
  int n = 0;
  if (base < 2)
    base = 10;
  if (value == 0) {
    tmp[n++] = '0';
  } else {
    while (value > 0) {
      tmp[n++] = digits[value % base];
      value /= base;
    }
  }
  for (int i = 0; i < n; i++)
    out[i] = tmp[n - 1 - i];
  out[n] = 0;
}

static void sx_put_float(sx_out *o, double value, int precision,
                         int force_decimal) {
  int negative = 0;
  if (value < 0) {
    negative = 1;
    value = -value;
  }
  if (precision < 0)
    precision = 6;
  if (precision > 18)
    precision = 18;
  sx_u64 scale = 1;
  for (int i = 0; i < precision; i++)
    scale *= 10;
  sx_u64 whole = (sx_u64)value;
  double fraction = value - (double)whole;
  sx_u64 frac = (sx_u64)(fraction * (double)scale + 0.5);
  if (frac >= scale) {
    whole++;
    frac -= scale;
  }
  char digits[66];
  sx_utoa_base(whole, 10, 0, digits);
  if (negative)
    sx_putc(o, '-');
  sx_puts_out(o, digits);
  if (precision > 0 || force_decimal) {
    sx_putc(o, '.');
    char fdigits[66];
    sx_utoa_base(frac, 10, 0, fdigits);
    int flen = (int)strlen(fdigits);
    sx_put_repeat(o, '0', precision - flen);
    sx_puts_out(o, fdigits);
  }
}

static int sx_vformat(char *buf, sx_size_t cap, const char *fmt,
                      sx_va_list ap) {
  sx_out o;
  o.buf = buf;
  o.cap = cap ? cap : 1;
  o.len = 0;
  o.overflow = 0;
  if (!fmt)
    fmt = "";

  while (*fmt) {
    if (*fmt != '%') {
      sx_putc(&o, *fmt++);
      continue;
    }
    fmt++;
    if (*fmt == '%') {
      sx_putc(&o, '%');
      fmt++;
      continue;
    }

    int left = 0, zero = 0, plus = 0, space = 0, alt = 0;
    for (;;) {
      if (*fmt == '-') {
        left = 1;
        fmt++;
      } else if (*fmt == '0') {
        zero = 1;
        fmt++;
      } else if (*fmt == '+') {
        plus = 1;
        fmt++;
      } else if (*fmt == ' ') {
        space = 1;
        fmt++;
      } else if (*fmt == '#') {
        alt = 1;
        fmt++;
      } else
        break;
    }
    int width = 0;
    if (*fmt == '*') {
      width = __builtin_va_arg(ap, int);
      if (width < 0) {
        left = 1;
        width = -width;
      }
      fmt++;
    } else {
      while (*fmt >= '0' && *fmt <= '9') {
        width = width * 10 + (*fmt - '0');
        fmt++;
      }
    }
    int precision = -1;
    if (*fmt == '.') {
      fmt++;
      precision = 0;
      if (*fmt == '*') {
        precision = __builtin_va_arg(ap, int);
        fmt++;
      } else {
        while (*fmt >= '0' && *fmt <= '9') {
          precision = precision * 10 + (*fmt - '0');
          fmt++;
        }
      }
    }
    int length = 0; /* 1 = hh, 2 = h, 3 = l, 4 = ll, 5 = z, 6 = t, 7 = j */
    if (*fmt == 'h') {
      fmt++;
      if (*fmt == 'h') {
        length = 1;
        fmt++;
      } else
        length = 2;
    } else if (*fmt == 'l') {
      fmt++;
      if (*fmt == 'l') {
        length = 4;
        fmt++;
      } else
        length = 3;
    } else if (*fmt == 'z' || *fmt == 't' || *fmt == 'j') {
      length = (*fmt == 'z') ? 5 : ((*fmt == 't') ? 6 : 7);
      fmt++;
    }
    if (!*fmt)
      break;
    char conv = *fmt++;
    char tmp[128];
    const char *text = tmp;
    tmp[0] = 0;

    if (conv == 'd' || conv == 'i') {
      sx_i64 v;
      if (length == 4 || length == 5 || length == 6 || length == 7)
        v = __builtin_va_arg(ap, sx_i64);
      else
        v = (sx_i64) __builtin_va_arg(ap, int);
      int neg = v < 0;
      sx_u64 mag = neg ? (sx_u64)(-v) : (sx_u64)v;
      sx_utoa_base(mag, 10, 0, tmp);
      if (neg) {
        sx_size_t n = strlen(tmp);
        memmove(tmp + 1, tmp, n + 1);
        tmp[0] = '-';
      } else if (plus) {
        sx_size_t n = strlen(tmp);
        memmove(tmp + 1, tmp, n + 1);
        tmp[0] = '+';
      } else if (space) {
        sx_size_t n = strlen(tmp);
        memmove(tmp + 1, tmp, n + 1);
        tmp[0] = ' ';
      }
    } else if (conv == 'u' || conv == 'o' || conv == 'x' || conv == 'X') {
      sx_u64 v = (length == 4 || length == 5 || length == 6 || length == 7)
                     ? __builtin_va_arg(ap, sx_u64)
                     : (sx_u64) __builtin_va_arg(ap, unsigned int);
      unsigned base =
          (conv == 'o') ? 8 : ((conv == 'x' || conv == 'X') ? 16 : 10);
      sx_utoa_base(v, base, conv == 'X', tmp);
      if (alt && v != 0) {
        if (conv == 'o') {
          sx_size_t n = strlen(tmp);
          memmove(tmp + 1, tmp, n + 1);
          tmp[0] = '0';
        } else if (conv == 'x' || conv == 'X') {
          sx_size_t n = strlen(tmp);
          memmove(tmp + 2, tmp, n + 1);
          tmp[0] = '0';
          tmp[1] = 'x';
        }
      }
    } else if (conv == 'p') {
      sx_u64 v = (sx_u64)(sx_uintptr_t) __builtin_va_arg(ap, void *);
      sx_utoa_base(v, 16, 0, tmp);
      sx_size_t n = strlen(tmp);
      memmove(tmp + 2, tmp, n + 1);
      tmp[0] = '0';
      tmp[1] = 'x';
    } else if (conv == 'c') {
      tmp[0] = (char)__builtin_va_arg(ap, int);
      tmp[1] = 0;
    } else if (conv == 's') {
      text = __builtin_va_arg(ap, const char *);
      if (!text)
        text = "(null)";
      if (precision >= 0) {
        int i = 0;
        while (i < precision && text[i])
          i++;
        if (i < (int)sizeof(tmp)) {
          for (int j = 0; j < i; j++)
            tmp[j] = text[j];
          tmp[i] = 0;
          text = tmp;
        }
      }
    } else if (conv == 'f' || conv == 'F' || conv == 'g' || conv == 'G' ||
               conv == 'e' || conv == 'E') {
      double v = __builtin_va_arg(ap, double);
      int p = precision;
      if (conv == 'g' || conv == 'G') {
        if (p < 0)
          p = 6;
        if (p == 0)
          p = 1;
      } else {
        if (p < 0)
          p = 6;
      }
      sx_put_float(&o, v, p, alt);
      continue;
    } else {
      tmp[0] = '%';
      tmp[1] = conv;
      tmp[2] = 0;
    }

    int text_len = (int)strlen(text);
    int pad = width > text_len ? width - text_len : 0;
    if (!left && pad > 0) {
      sx_put_repeat(&o, zero ? '0' : ' ', pad);
    }
    sx_puts_out(&o, text);
    if (left && pad > 0) {
      sx_put_repeat(&o, ' ', pad);
    }
  }

  if (o.cap > 0) {
    sx_size_t end = o.len < o.cap ? o.len : o.cap - 1;
    o.buf[end] = 0;
  }
  return (int)o.len;
}

int vsnprintf(char *buf, sx_size_t cap, const char *fmt, sx_va_list ap) {
  return sx_vformat(buf, cap, fmt, ap);
}

int snprintf(char *buf, sx_size_t cap, const char *fmt, ...) {
  sx_va_list ap;
  __builtin_va_start(ap, fmt);
  int n = sx_vformat(buf, cap, fmt, ap);
  __builtin_va_end(ap);
  return n;
}

int vsprintf(char *buf, const char *fmt, sx_va_list ap) {
  return sx_vformat(buf, (sx_size_t)-1, fmt, ap);
}

int sprintf(char *buf, const char *fmt, ...) {
  sx_va_list ap;
  __builtin_va_start(ap, fmt);
  int n = sx_vformat(buf, (sx_size_t)-1, fmt, ap);
  __builtin_va_end(ap);
  return n;
}

/* ------------------------------------------------------------------------- */
/* Platform layer                                                            */
/* ------------------------------------------------------------------------- */

#if defined(SX_LINUX)

/* Linux syscall numbers (x86_64 / aarch64). */
#if defined(__x86_64__)
#define SX_SYS_read 0
#define SX_SYS_write 1
#define SX_SYS_openat 257
#define SX_SYS_close 3
#define SX_SYS_lseek 8
#define SX_SYS_mmap 9
#define SX_SYS_munmap 11
#define SX_SYS_mprotect 10
#define SX_SYS_exit_group 231
#define SX_SYS_exit 60
#define SX_SYS_clock_gettime 228
#define SX_SYS_getrandom 318
#define SX_SYS_clone 56
#define SX_SYS_futex 202
#define SX_SYS_socket 41
#define SX_SYS_connect 42
#define SX_SYS_accept 43
#define SX_SYS_sendto 44
#define SX_SYS_recvfrom 45
#define SX_SYS_bind 49
#define SX_SYS_listen 50
#define SX_SYS_setsockopt 54
#define SX_SYS_getsockopt 55
#define SX_SYS_poll 7
#define SX_SYS_wait4 61
#define SX_SYS_execve 59
#define SX_SYS_pipe2 293
#define SX_SYS_dup 32
#define SX_SYS_dup2 33
#define SX_SYS_getdents64 217
#define SX_SYS_newfstatat 262
#define SX_SYS_faccessat 269
#define SX_SYS_mkdirat 258
#define SX_SYS_unlinkat 263
#define SX_SYS_renameat 264
#define SX_SYS_getcwd 79
#define SX_SYS_chdir 80
#define SX_SYS_readlinkat 267
#define SX_SYS_uname 63
#define SX_SYS_nanosleep 35
#define SX_SYS_ioctl 16
#define SX_SYS_rt_sigaction 13
#define SX_SYS_kill 62
#else
#define SX_SYS_read 63
#define SX_SYS_write 64
#define SX_SYS_openat 56
#define SX_SYS_close 57
#define SX_SYS_lseek 62
#define SX_SYS_mmap 222
#define SX_SYS_munmap 215
#define SX_SYS_mprotect 226
#define SX_SYS_exit_group 94
#define SX_SYS_exit 93
#define SX_SYS_clock_gettime 113
#define SX_SYS_getrandom 278
#define SX_SYS_clone 220
#define SX_SYS_futex 98
#define SX_SYS_socket 198
#define SX_SYS_connect 203
#define SX_SYS_accept 202
#define SX_SYS_sendto 206
#define SX_SYS_recvfrom 207
#define SX_SYS_bind 200
#define SX_SYS_listen 201
#define SX_SYS_setsockopt 208
#define SX_SYS_getsockopt 209
#define SX_SYS_poll 7
#define SX_SYS_wait4 260
#define SX_SYS_execve 221
#define SX_SYS_pipe2 59
#define SX_SYS_dup 23
#define SX_SYS_dup2 24
#define SX_SYS_getdents64 61
#define SX_SYS_newfstatat 79
#define SX_SYS_faccessat 48
#define SX_SYS_mkdirat 34
#define SX_SYS_unlinkat 35
#define SX_SYS_renameat 38
#define SX_SYS_getcwd 17
#define SX_SYS_chdir 49
#define SX_SYS_readlinkat 78
#define SX_SYS_uname 160
#define SX_SYS_nanosleep 101
#define SX_SYS_ioctl 29
#define SX_SYS_rt_sigaction 134
#define SX_SYS_kill 129
#endif

#if defined(__x86_64__)
static sx_i64 sx_sc0(sx_i64 n) {
  sx_i64 r;
  __asm__ volatile("syscall" : "=a"(r) : "a"(n) : "rcx", "r11", "memory");
  return r;
}
static sx_i64 sx_sc1(sx_i64 n, sx_i64 a1) {
  sx_i64 r;
  __asm__ volatile("syscall"
                   : "=a"(r)
                   : "a"(n), "D"(a1)
                   : "rcx", "r11", "memory");
  return r;
}
static sx_i64 sx_sc2(sx_i64 n, sx_i64 a1, sx_i64 a2) {
  sx_i64 r;
  __asm__ volatile("syscall"
                   : "=a"(r)
                   : "a"(n), "D"(a1), "S"(a2)
                   : "rcx", "r11", "memory");
  return r;
}
static sx_i64 sx_sc3(sx_i64 n, sx_i64 a1, sx_i64 a2, sx_i64 a3) {
  sx_i64 r;
  __asm__ volatile("syscall"
                   : "=a"(r)
                   : "a"(n), "D"(a1), "S"(a2), "d"(a3)
                   : "rcx", "r11", "memory");
  return r;
}
static sx_i64 sx_sc4(sx_i64 n, sx_i64 a1, sx_i64 a2, sx_i64 a3, sx_i64 a4) {
  sx_i64 r;
  register sx_i64 r10 __asm__("r10") = a4;
  __asm__ volatile("syscall"
                   : "=a"(r)
                   : "a"(n), "D"(a1), "S"(a2), "d"(a3), "r"(r10)
                   : "rcx", "r11", "memory");
  return r;
}
static sx_i64 sx_sc5(sx_i64 n, sx_i64 a1, sx_i64 a2, sx_i64 a3, sx_i64 a4,
                     sx_i64 a5) {
  sx_i64 r;
  register sx_i64 r10 __asm__("r10") = a4;
  register sx_i64 r8 __asm__("r8") = a5;
  __asm__ volatile("syscall"
                   : "=a"(r)
                   : "a"(n), "D"(a1), "S"(a2), "d"(a3), "r"(r10), "r"(r8)
                   : "rcx", "r11", "memory");
  return r;
}
static sx_i64 sx_sc6(sx_i64 n, sx_i64 a1, sx_i64 a2, sx_i64 a3, sx_i64 a4,
                     sx_i64 a5, sx_i64 a6) {
  sx_i64 r;
  register sx_i64 r10 __asm__("r10") = a4;
  register sx_i64 r8 __asm__("r8") = a5;
  register sx_i64 r9 __asm__("r9") = a6;
  __asm__ volatile("syscall"
                   : "=a"(r)
                   : "a"(n), "D"(a1), "S"(a2), "d"(a3), "r"(r10), "r"(r8),
                     "r"(r9)
                   : "rcx", "r11", "memory");
  return r;
}
#else
static sx_i64 sx_sc0(sx_i64 n) {
  register sx_i64 x8 __asm__("x8") = n;
  register sx_i64 x0 __asm__("x0");
  __asm__ volatile("svc 0" : "=r"(x0) : "r"(x8) : "memory");
  return x0;
}
static sx_i64 sx_sc1(sx_i64 n, sx_i64 a1) {
  register sx_i64 x8 __asm__("x8") = n;
  register sx_i64 x0 __asm__("x0") = a1;
  __asm__ volatile("svc 0" : "+r"(x0) : "r"(x8) : "memory");
  return x0;
}
static sx_i64 sx_sc2(sx_i64 n, sx_i64 a1, sx_i64 a2) {
  register sx_i64 x8 __asm__("x8") = n;
  register sx_i64 x0 __asm__("x0") = a1;
  register sx_i64 x1 __asm__("x1") = a2;
  __asm__ volatile("svc 0" : "+r"(x0) : "r"(x8), "r"(x1) : "memory");
  return x0;
}
static sx_i64 sx_sc3(sx_i64 n, sx_i64 a1, sx_i64 a2, sx_i64 a3) {
  register sx_i64 x8 __asm__("x8") = n;
  register sx_i64 x0 __asm__("x0") = a1;
  register sx_i64 x1 __asm__("x1") = a2;
  register sx_i64 x2 __asm__("x2") = a3;
  __asm__ volatile("svc 0" : "+r"(x0) : "r"(x8), "r"(x1), "r"(x2) : "memory");
  return x0;
}
static sx_i64 sx_sc4(sx_i64 n, sx_i64 a1, sx_i64 a2, sx_i64 a3, sx_i64 a4) {
  register sx_i64 x8 __asm__("x8") = n;
  register sx_i64 x0 __asm__("x0") = a1;
  register sx_i64 x1 __asm__("x1") = a2;
  register sx_i64 x2 __asm__("x2") = a3;
  register sx_i64 x3 __asm__("x3") = a4;
  __asm__ volatile("svc 0"
                   : "+r"(x0)
                   : "r"(x8), "r"(x1), "r"(x2), "r"(x3)
                   : "memory");
  return x0;
}
static sx_i64 sx_sc5(sx_i64 n, sx_i64 a1, sx_i64 a2, sx_i64 a3, sx_i64 a4,
                     sx_i64 a5) {
  register sx_i64 x8 __asm__("x8") = n;
  register sx_i64 x0 __asm__("x0") = a1;
  register sx_i64 x1 __asm__("x1") = a2;
  register sx_i64 x2 __asm__("x2") = a3;
  register sx_i64 x3 __asm__("x3") = a4;
  register sx_i64 x4 __asm__("x4") = a5;
  __asm__ volatile("svc 0"
                   : "+r"(x0)
                   : "r"(x8), "r"(x1), "r"(x2), "r"(x3), "r"(x4)
                   : "memory");
  return x0;
}
static sx_i64 sx_sc6(sx_i64 n, sx_i64 a1, sx_i64 a2, sx_i64 a3, sx_i64 a4,
                     sx_i64 a5, sx_i64 a6) {
  register sx_i64 x8 __asm__("x8") = n;
  register sx_i64 x0 __asm__("x0") = a1;
  register sx_i64 x1 __asm__("x1") = a2;
  register sx_i64 x2 __asm__("x2") = a3;
  register sx_i64 x3 __asm__("x3") = a4;
  register sx_i64 x4 __asm__("x4") = a5;
  register sx_i64 x5 __asm__("x5") = a6;
  __asm__ volatile("svc 0"
                   : "+r"(x0)
                   : "r"(x8), "r"(x1), "r"(x2), "r"(x3), "r"(x4), "r"(x5)
                   : "memory");
  return x0;
}
#endif

#define SX_AT_FDCWD (-100)
#define SX_O_RDONLY 0
#define SX_O_WRONLY 1
#define SX_O_RDWR 2
#define SX_O_CREAT 0100
#define SX_O_TRUNC 01000
#define SX_O_APPEND 02000
#define SX_O_DIRECTORY 00200000
#define SX_PROT_READ 1
#define SX_PROT_WRITE 2
#define SX_MAP_PRIVATE 2
#define SX_MAP_ANONYMOUS 0x20

char **environ = SX_NULL;
static char **sx_envp = SX_NULL;
static int sx_argc = 0;
static char **sx_argv = SX_NULL;
void __sx_set_envp(char **envp) {
  sx_envp = envp;
  environ = envp;
}
void __sx_set_argc_argv(int argc, char **argv) {
  sx_argc = argc;
  sx_argv = argv;
}

static sx_i64 sx_write_fd(int fd, const void *buf, sx_size_t n) {
  return sx_sc3(SX_SYS_write, fd, (sx_i64)(sx_uintptr_t)buf, (sx_i64)n);
}
static sx_i64 sx_read_fd(int fd, void *buf, sx_size_t n) {
  return sx_sc3(SX_SYS_read, fd, (sx_i64)(sx_uintptr_t)buf, (sx_i64)n);
}
static int sx_open_path(const char *path, int flags, int mode) {
  return (int)sx_sc4(SX_SYS_openat, SX_AT_FDCWD, (sx_i64)(sx_uintptr_t)path,
                     flags, mode);
}
static int sx_close_fd(int fd) { return (int)sx_sc1(SX_SYS_close, fd); }

void exit(int code) {
  sx_sc1(SX_SYS_exit_group, code);
  for (;;) {
  }
}
void _exit(int code) { exit(code); }

/* Simple heap: 1 MiB chunks plus individually mapped large blocks. */
typedef struct sx_chunk {
  struct sx_chunk *next;
  sx_size_t used;
  sx_size_t cap;
} sx_chunk;

static sx_chunk *sx_heap = SX_NULL;
#define SX_CHUNK_SIZE (1024 * 1024)

static void *sx_map(sx_size_t n) {
  sx_i64 r = sx_sc6(SX_SYS_mmap, 0, (sx_i64)n, SX_PROT_READ | SX_PROT_WRITE,
                    SX_MAP_PRIVATE | SX_MAP_ANONYMOUS, -1, 0);
  if (r < 0 && r > -4096)
    return SX_NULL;
  return (void *)(sx_uintptr_t)r;
}

static void *sx_alloc_raw(sx_size_t n) {
  n = (n + 15u) & ~(sx_size_t)15u;
  if (n == 0)
    n = 16;
  if (n >= SX_CHUNK_SIZE / 4) {
    void *p = sx_map(n);
    return p;
  }
  if (!sx_heap || sx_heap->used + n > sx_heap->cap) {
    sx_size_t cap = SX_CHUNK_SIZE;
    if (n + sizeof(sx_chunk) > cap)
      cap = n + sizeof(sx_chunk);
    sx_chunk *c = (sx_chunk *)sx_map(cap);
    if (!c)
      return SX_NULL;
    c->used = sizeof(sx_chunk);
    c->cap = cap;
    c->next = sx_heap;
    sx_heap = c;
  }
  void *p = (void *)((unsigned char *)sx_heap + sx_heap->used);
  sx_heap->used += n;
  return p;
}

void *malloc(sx_size_t n) {
  sx_size_t *p = (sx_size_t *)sx_alloc_raw(n + 16);
  if (!p)
    return SX_NULL;
  p[0] = n;
  p[1] = 0x53585358u;
  return (void *)(p + 2);
}
void *calloc(sx_size_t count, sx_size_t size) {
  sx_size_t n = count * size;
  void *p = malloc(n);
  if (p)
    memset(p, 0, n);
  return p;
}
void free(void *ptr) {
  (void)ptr; /* Small allocations live in chunks; arenas release in bulk. */
}
void *realloc(void *ptr, sx_size_t n) {
  if (!ptr)
    return malloc(n);
  if (n == 0) {
    free(ptr);
    return SX_NULL;
  }
  sx_size_t *h = ((sx_size_t *)ptr) - 2;
  sx_size_t old = h[0];
  void *p = malloc(n);
  if (!p)
    return SX_NULL;
  memcpy(p, ptr, old < n ? old : n);
  return p;
}

/* Files */
typedef struct sx_file {
  int fd;
  int mode;
  int eof;
  int ungot;
} FILE;

static FILE sx_stdin_obj = {0, 0, 0, -1};
static FILE sx_stdout_obj = {1, 1, 0, -1};
static FILE sx_stderr_obj = {2, 1, 0, -1};
FILE *stdin = &sx_stdin_obj;
FILE *stdout = &sx_stdout_obj;
FILE *stderr = &sx_stderr_obj;

FILE *fdopen(int fd, const char *mode) {
  FILE *f = (FILE *)calloc(1, sizeof(FILE));
  if (!f)
    return SX_NULL;
  f->fd = fd;
  f->ungot = -1;
  if (mode && mode[0] == 'w')
    f->mode = 1;
  else if (mode && mode[0] == 'a')
    f->mode = 2;
  return f;
}

FILE *fopen(const char *path, const char *mode) {
  int flags = SX_O_RDONLY;
  if (!mode)
    mode = "r";
  if (mode[0] == 'w')
    flags = SX_O_WRONLY | SX_O_CREAT | SX_O_TRUNC;
  else if (mode[0] == 'a')
    flags = SX_O_WRONLY | SX_O_CREAT | SX_O_APPEND;
  else if (mode[0] == 'r' && mode[1] == '+')
    flags = SX_O_RDWR;
  else if (mode[0] == 'w' && mode[1] == '+')
    flags = SX_O_RDWR | SX_O_CREAT | SX_O_TRUNC;
  else if (mode[0] == 'a' && mode[1] == '+')
    flags = SX_O_RDWR | SX_O_CREAT | SX_O_APPEND;
  int fd = sx_open_path(path, flags, 0666);
  if (fd < 0)
    return SX_NULL;
  FILE *f = fdopen(fd, mode);
  if (!f)
    sx_close_fd(fd);
  return f;
}

int fclose(FILE *f) {
  if (!f)
    return -1;
  if (f != stdin && f != stdout && f != stderr) {
    sx_close_fd(f->fd);
    free(f);
  }
  return 0;
}

sx_size_t fread(void *ptr, sx_size_t size, sx_size_t nmemb, FILE *f) {
  if (!f || !ptr)
    return 0;
  sx_size_t total = size * nmemb;
  char *p = (char *)ptr;
  sx_size_t done = 0;
  if (f->ungot >= 0 && total > 0) {
    p[0] = (char)f->ungot;
    f->ungot = -1;
    done = 1;
  }
  while (done < total) {
    sx_i64 n = sx_read_fd(f->fd, p + done, total - done);
    if (n <= 0) {
      f->eof = 1;
      break;
    }
    done += (sx_size_t)n;
  }
  return size ? done / size : 0;
}

sx_size_t fwrite(const void *ptr, sx_size_t size, sx_size_t nmemb, FILE *f) {
  if (!f || !ptr)
    return 0;
  sx_size_t total = size * nmemb;
  const char *p = (const char *)ptr;
  sx_size_t done = 0;
  while (done < total) {
    sx_i64 n = sx_write_fd(f->fd, p + done, total - done);
    if (n <= 0)
      break;
    done += (sx_size_t)n;
  }
  return size ? done / size : 0;
}

int fseek(FILE *f, long offset, int whence) {
  if (!f)
    return -1;
  sx_i64 r = sx_sc3(SX_SYS_lseek, f->fd, offset, whence);
  f->eof = 0;
  return r < 0 ? -1 : 0;
}
long ftell(FILE *f) {
  if (!f)
    return -1;
  return (long)sx_sc3(SX_SYS_lseek, f->fd, 0, 1);
}
void rewind(FILE *f) {
  if (f)
    fseek(f, 0, 0);
}
int feof(FILE *f) { return f ? f->eof : 0; }
int fflush(FILE *f) {
  (void)f;
  return 0;
}
int fgetc(FILE *f) {
  unsigned char c;
  if (!f)
    return -1;
  if (f->ungot >= 0) {
    int r = f->ungot;
    f->ungot = -1;
    return r;
  }
  if (sx_read_fd(f->fd, &c, 1) != 1) {
    f->eof = 1;
    return -1;
  }
  return (int)c;
}
int getchar(void) { return fgetc(stdin); }
int fputc(int c, FILE *f) {
  unsigned char ch = (unsigned char)c;
  if (!f || sx_write_fd(f->fd, &ch, 1) != 1)
    return -1;
  return ch;
}
int putchar(int c) { return fputc(c, stdout); }
int ungetc(int c, FILE *f) {
  if (!f)
    return -1;
  f->ungot = c & 0xff;
  return c;
}
char *fgets(char *buf, int n, FILE *f) {
  if (!buf || n <= 0 || !f)
    return SX_NULL;
  int i = 0;
  while (i < n - 1) {
    int c = fgetc(f);
    if (c < 0)
      break;
    buf[i++] = (char)c;
    if (c == '\n')
      break;
  }
  if (i == 0)
    return SX_NULL;
  buf[i] = 0;
  return buf;
}

int printf(const char *fmt, ...) {
  char buf[4096];
  sx_va_list ap;
  __builtin_va_start(ap, fmt);
  int n = sx_vformat(buf, sizeof(buf), fmt, ap);
  __builtin_va_end(ap);
  if (n > 0)
    sx_write_fd(1, buf,
                (sx_size_t)n < sizeof(buf) ? (sx_size_t)n : sizeof(buf) - 1);
  return n;
}
int fprintf(FILE *f, const char *fmt, ...) {
  char buf[4096];
  sx_va_list ap;
  __builtin_va_start(ap, fmt);
  int n = sx_vformat(buf, sizeof(buf), fmt, ap);
  __builtin_va_end(ap);
  if (n > 0 && f)
    sx_write_fd(f->fd, buf,
                (sx_size_t)n < sizeof(buf) ? (sx_size_t)n : sizeof(buf) - 1);
  return n;
}
int dprintf(int fd, const char *fmt, ...) {
  char buf[4096];
  sx_va_list ap;
  __builtin_va_start(ap, fmt);
  int n = sx_vformat(buf, sizeof(buf), fmt, ap);
  __builtin_va_end(ap);
  if (n > 0)
    sx_write_fd(fd, buf,
                (sx_size_t)n < sizeof(buf) ? (sx_size_t)n : sizeof(buf) - 1);
  return n;
}
int puts(const char *s) {
  if (!s)
    s = "(null)";
  sx_size_t n = strlen(s);
  sx_write_fd(1, s, n);
  sx_write_fd(1, "\n", 1);
  return (int)n + 1;
}
int fputs(const char *s, FILE *f) {
  if (!s || !f)
    return -1;
  sx_size_t n = strlen(s);
  sx_write_fd(f->fd, s, n);
  return (int)n;
}
void perror(const char *s) {
  if (s) {
    dprintf(2, "%s: error\n", s);
  } else {
    dprintf(2, "error\n");
  }
}

/* Environment / paths */
/* Directory iteration */
typedef struct sx_dir {
  int fd;
  int buf_pos;
  int buf_len;
  unsigned char buf[8192];
} DIR;

typedef struct sx_dirent64 {
  sx_u64 d_ino;
  sx_i64 d_off;
  sx_u16 d_reclen;
  sx_u8 d_type;
  char d_name[];
} sx_dirent64;

DIR *opendir(const char *path) {
  int fd = sx_open_path(path, SX_O_RDONLY | SX_O_DIRECTORY, 0);
  if (fd < 0)
    return SX_NULL;
  DIR *d = (DIR *)calloc(1, sizeof(DIR));
  if (!d) {
    sx_close_fd(fd);
    return SX_NULL;
  }
  d->fd = fd;
  return d;
}
void *readdir(DIR *d) {
  if (!d)
    return SX_NULL;
  if (d->buf_pos >= d->buf_len) {
    sx_i64 n = sx_sc3(SX_SYS_getdents64, d->fd, (sx_i64)(sx_uintptr_t)d->buf,
                      sizeof(d->buf));
    if (n <= 0)
      return SX_NULL;
    d->buf_pos = 0;
    d->buf_len = (int)n;
  }
  sx_dirent64 *e = (sx_dirent64 *)(d->buf + d->buf_pos);
  if (e->d_reclen == 0)
    return SX_NULL;
  d->buf_pos += e->d_reclen;
  return e;
}
int closedir(DIR *d) {
  if (!d)
    return -1;
  sx_close_fd(d->fd);
  free(d);
  return 0;
}

/* Time */
typedef struct {
  sx_i64 tv_sec;
  sx_i64 tv_usec;
} sx_timeval;
typedef struct {
  sx_i64 tv_sec;
  sx_i64 tv_nsec;
} sx_timespec;
typedef struct {
  sx_i32 tm_sec;
  sx_i32 tm_min;
  sx_i32 tm_hour;
  sx_i32 tm_mday;
  sx_i32 tm_mon;
  sx_i32 tm_year;
  sx_i32 tm_wday;
  sx_i32 tm_yday;
  sx_i32 tm_isdst;
  sx_i64 tm_gmtoff;
  const char *tm_zone;
} sx_tm;

static sx_tm sx_tm_storage;
static const int sx_days_before_month[12] = {0,   31,  59,  90,  120, 151,
                                             181, 212, 243, 273, 304, 334};
/* Random */
/* Processes */
int fork(void) {
  return (int)sx_sc0(SX_SYS_clone); /* not used; clone needs flags on Linux */
}
static sx_i64 sx_clone_process(void) {
#if defined(__x86_64__)
  /* clone(SIGCHLD, 0, 0, 0, 0) */
  return sx_sc5(SX_SYS_clone, 17, 0, 0, 0, 0);
#else
  return sx_sc5(SX_SYS_clone, 17, 0, 0, 0, 0);
#endif
}
int pipe(int fds[2]) {
  return (int)sx_sc2(SX_SYS_pipe2, (sx_i64)(sx_uintptr_t)fds, 0);
}
int dup2(int oldfd, int newfd) {
  return (int)sx_sc2(SX_SYS_dup2, oldfd, newfd);
}
int dup(int fd) { return (int)sx_sc1(SX_SYS_dup, fd); }
int execve(const char *path, char *const argv[], char *const envp[]) {
  return (int)sx_sc3(SX_SYS_execve, (sx_i64)(sx_uintptr_t)path,
                     (sx_i64)(sx_uintptr_t)argv, (sx_i64)(sx_uintptr_t)envp);
}
int waitpid(int pid, int *status, int options) {
  return (int)sx_sc4(SX_SYS_wait4, pid, (sx_i64)(sx_uintptr_t)status, options,
                     0);
}
int system(const char *cmd) {
  if (!cmd)
    return 1;
  sx_i64 pid = sx_clone_process();
  if (pid == 0) {
    char *argv[4];
    argv[0] = (char *)"/bin/sh";
    argv[1] = (char *)"-c";
    argv[2] = (char *)cmd;
    argv[3] = SX_NULL;
    execve("/bin/sh", argv, environ);
    exit(127);
  }
  if (pid < 0)
    return -1;
  int status = 0;
  waitpid((int)pid, &status, 0);
  return status;
}
FILE *popen(const char *cmd, const char *mode) {
  int fds[2];
  if (pipe(fds) != 0)
    return SX_NULL;
  sx_i64 pid = sx_clone_process();
  if (pid == 0) {
    if (mode && mode[0] == 'r') {
      dup2(fds[1], 1);
    } else {
      dup2(fds[0], 0);
    }
    sx_close_fd(fds[0]);
    sx_close_fd(fds[1]);
    char *argv[4];
    argv[0] = (char *)"/bin/sh";
    argv[1] = (char *)"-c";
    argv[2] = (char *)cmd;
    argv[3] = SX_NULL;
    execve("/bin/sh", argv, environ);
    exit(127);
  }
  if (pid < 0) {
    sx_close_fd(fds[0]);
    sx_close_fd(fds[1]);
    return SX_NULL;
  }
  FILE *f;
  if (mode && mode[0] == 'r') {
    sx_close_fd(fds[1]);
    f = fdopen(fds[0], "r");
  } else {
    sx_close_fd(fds[0]);
    f = fdopen(fds[1], "w");
  }
  if (f) {
    /* Store the child pid in the unused mode field. */
    f->mode = (int)pid;
  }
  return f;
}
int pclose(FILE *f) {
  if (!f)
    return -1;
  int pid = f->mode;
  fclose(f);
  int status = 0;
  waitpid(pid, &status, 0);
  return status;
}

/* Signals (software dispatch; real asynchronous delivery is best effort). */
static void (*sx_signal_handlers[65])(int);
void (*signal(int sig, void (*handler)(int)))(int) {
  void (*old)(int) = SX_NULL;
  if (sig > 0 && sig < 65) {
    old = sx_signal_handlers[sig];
    sx_signal_handlers[sig] = handler;
  }
  return old;
}
int raise(int sig) {
  if (sig > 0 && sig < 65 && sx_signal_handlers[sig]) {
    sx_signal_handlers[sig](sig);
    return 0;
  }
  return (int)sx_sc2(SX_SYS_kill, 0, sig);
}

/* struct utsname: six 65-byte fields is the Linux ABI. */
struct sx_utsname {
  char sysname[65];
  char nodename[65];
  char release[65];
  char version[65];
  char machine[65];
  char domainname[65];
};
extern long __sx_clone_thread(long flags, void *stack_top, void *entry,
                              void *arg);

/* Sockets: minimal syscall bridge. */
typedef unsigned sx_socklen;
int socket(int domain, int type, int protocol) {
  return (int)sx_sc3(SX_SYS_socket, domain, type, protocol);
}
int bind(int fd, const void *addr, sx_socklen len) {
  return (int)sx_sc3(SX_SYS_bind, fd, (sx_i64)(sx_uintptr_t)addr, len);
}
int listen(int fd, int backlog) {
  return (int)sx_sc2(SX_SYS_listen, fd, backlog);
}
int accept(int fd, void *addr, sx_socklen *len) {
  return (int)sx_sc3(SX_SYS_accept, fd, (sx_i64)(sx_uintptr_t)addr,
                     (sx_i64)(sx_uintptr_t)len);
}
int connect(int fd, const void *addr, sx_socklen len) {
  return (int)sx_sc3(SX_SYS_connect, fd, (sx_i64)(sx_uintptr_t)addr, len);
}
sx_ssize_t send(int fd, const void *buf, sx_size_t len, int flags) {
  return (sx_ssize_t)sx_sc6(SX_SYS_sendto, fd, (sx_i64)(sx_uintptr_t)buf,
                            (sx_i64)len, flags, 0, 0);
}
sx_ssize_t recv(int fd, void *buf, sx_size_t len, int flags) {
  return (sx_ssize_t)sx_sc6(SX_SYS_recvfrom, fd, (sx_i64)(sx_uintptr_t)buf,
                            (sx_i64)len, flags, 0, 0);
}
int sendto(int fd, const void *buf, sx_size_t len, int flags, const void *addr,
           sx_socklen alen) {
  return (int)sx_sc6(SX_SYS_sendto, fd, (sx_i64)(sx_uintptr_t)buf, (sx_i64)len,
                     flags, (sx_i64)(sx_uintptr_t)addr, alen);
}
int recvfrom(int fd, void *buf, sx_size_t len, int flags, void *addr,
             sx_socklen *alen) {
  return (int)sx_sc6(SX_SYS_recvfrom, fd, (sx_i64)(sx_uintptr_t)buf,
                     (sx_i64)len, flags, (sx_i64)(sx_uintptr_t)addr,
                     (sx_i64)(sx_uintptr_t)alen);
}
int setsockopt(int fd, int level, int opt, const void *val, sx_socklen len) {
  return (int)sx_sc5(SX_SYS_setsockopt, fd, level, opt,
                     (sx_i64)(sx_uintptr_t)val, len);
}
int getsockopt(int fd, int level, int opt, void *val, sx_socklen *len) {
  return (int)sx_sc5(SX_SYS_getsockopt, fd, level, opt,
                     (sx_i64)(sx_uintptr_t)val, (sx_i64)(sx_uintptr_t)len);
}
unsigned short htons(unsigned short v) {
  return (unsigned short)((v << 8) | (v >> 8));
}
unsigned short ntohs(unsigned short v) { return htons(v); }
unsigned int htonl(unsigned int v) {
  return ((v & 0xffu) << 24) | ((v & 0xff00u) << 8) | ((v >> 8) & 0xff00u) |
         ((v >> 24) & 0xffu);
}
unsigned int ntohl(unsigned int v) { return htonl(v); }
int poll(void *fds, unsigned long n, int timeout) {
  return (int)sx_sc3(SX_SYS_poll, (sx_i64)(sx_uintptr_t)fds, (sx_i64)n,
                     timeout);
}

/* Additional POSIX-compatible entry points used by the standard library. */
sx_ssize_t write(int fd, const void *buf, sx_size_t n) {
  return (sx_ssize_t)sx_write_fd(fd, buf, n);
}
sx_ssize_t read(int fd, void *buf, sx_size_t n) {
  return (sx_ssize_t)sx_read_fd(fd, buf, n);
}
int close(int fd) { return sx_close_fd(fd); }
int open(const char *path, int flags, ...) {
  int mode = 0666;
  return sx_open_path(path, flags, mode);
}
int creat(const char *path, int mode) {
  return sx_open_path(path, SX_O_WRONLY | SX_O_CREAT | SX_O_TRUNC, mode);
}
int fcntl(int fd, int cmd, ...) {
#if defined(__x86_64__)
  return (int)sx_sc3(72, fd, cmd, 0);
#else
  return (int)sx_sc3(25, fd, cmd, 0);
#endif
}
int ferror(FILE *f) {
  (void)f;
  return 0;
}
int clearerr(FILE *f) {
  if (f)
    f->eof = 0;
  return 0;
}
int fileno(FILE *f) { return f ? f->fd : -1; }

char *strdup(const char *s) {
  if (!s)
    return SX_NULL;
  sx_size_t n = strlen(s) + 1;
  char *p = (char *)malloc(n);
  if (p)
    memcpy(p, s, n);
  return p;
}
char *strtok_r(char *s, const char *delim, char **saveptr) {
  if (!delim || !saveptr)
    return SX_NULL;
  if (!s)
    s = *saveptr;
  if (!s)
    return SX_NULL;
  while (*s && strchr(delim, *s))
    s++;
  if (!*s) {
    *saveptr = s;
    return SX_NULL;
  }
  char *start = s;
  while (*s && !strchr(delim, *s))
    s++;
  if (*s) {
    *s = 0;
    s++;
  }
  *saveptr = s;
  return start;
}

static int sx_errno_storage = 0;
int *__errno_location(void) { return &sx_errno_storage; }

int sigaction(int sig, const void *act, void *old) {
  (void)sig;
#if defined(__x86_64__)
  return (int)sx_sc4(SX_SYS_rt_sigaction, sig, (sx_i64)(sx_uintptr_t)act,
                     (sx_i64)(sx_uintptr_t)old, 8);
#else
  return (int)sx_sc4(SX_SYS_rt_sigaction, sig, (sx_i64)(sx_uintptr_t)act,
                     (sx_i64)(sx_uintptr_t)old, 8);
#endif
}
int sigemptyset(void *set) {
  if (set)
    memset(set, 0, 128);
  return 0;
}
int sigaddset(void *set, int sig) {
  if (set && sig > 0 && sig < 1024) {
    unsigned char *p = (unsigned char *)set;
    p[(sig - 1) / 8] |= (unsigned char)(1u << ((sig - 1) % 8));
  }
  return 0;
}

/* The Spectre stdlib Stat shape is intentionally compact:
 * { ino: u64, mode: u32, uid: u32, size: i64 }.  Translate the kernel
 * stat buffer into it instead of copying the platform struct wholesale. */
typedef struct {
  sx_u64 st_ino;
  sx_u32 st_mode;
  sx_u32 st_uid;
  sx_i64 st_size;
} sx_compact_stat;

static void sx_fill_compact_stat(const unsigned char *raw, void *dst) {
  if (!dst)
    return;
  sx_compact_stat out;
  out.st_ino = *(const sx_u64 *)(raw + 8);
#if defined(__x86_64__)
  out.st_mode = *(const sx_u32 *)(raw + 24);
  out.st_uid = *(const sx_u32 *)(raw + 28);
#else
  out.st_mode = *(const sx_u32 *)(raw + 16);
  out.st_uid = *(const sx_u32 *)(raw + 24);
#endif
  out.st_size = *(const sx_i64 *)(raw + 48);
  memcpy(dst, &out, sizeof(out));
}

int inet_pton(int af, const char *src, void *dst) {
  (void)af;
  unsigned char *out = (unsigned char *)dst;
  int part = 0, value = 0, digits = 0;
  for (const char *p = src;; p++) {
    if (*p >= '0' && *p <= '9') {
      value = value * 10 + (*p - '0');
      digits++;
      if (value > 255)
        return 0;
    } else if (*p == '.' || *p == 0) {
      if (digits == 0 || part >= 4)
        return 0;
      out[part++] = (unsigned char)value;
      value = 0;
      digits = 0;
      if (*p == 0)
        break;
    } else {
      return 0;
    }
  }
  return part == 4 ? 1 : 0;
}
const char *inet_ntop(int af, const void *src, char *dst, sx_size_t size) {
  (void)af;
  if (!dst || size < 16)
    return SX_NULL;
  const unsigned char *b = (const unsigned char *)src;
  int n = snprintf(dst, size, "%u.%u.%u.%u", b[0], b[1], b[2], b[3]);
  return n > 0 ? dst : SX_NULL;
}

void __stack_chk_fail(void) { exit(127); }
int posix_spawn_file_actions_addtcsetpgrp_np(void *a, int fd) {
  (void)a;
  (void)fd;
  return -1;
}
int posix_spawn_file_actions_addclosefrom_np(void *a, int fd) {
  (void)a;
  (void)fd;
  return -1;
}

#else /* SX_WINDOWS */

/* ------------------------------------------------------------------------- */
/* Windows                                                                   */
/* ------------------------------------------------------------------------- */

typedef void *HANDLE;
typedef void *HMODULE;
typedef unsigned long DWORD;
typedef int BOOL;
typedef unsigned int UINT;
typedef unsigned long long ULONGLONG;
typedef long long LONGLONG;
typedef unsigned short WORD;
typedef unsigned char BYTE;
typedef long LONG;
typedef sx_size_t SIZE_T;

#define SX_INVALID_HANDLE ((HANDLE)(sx_intptr_t) - 1)
#define SX_GENERIC_READ 0x80000000u
#define SX_GENERIC_WRITE 0x40000000u
#define SX_FILE_SHARE_READ 1u
#define SX_FILE_SHARE_WRITE 2u
#define SX_OPEN_EXISTING 3u
#define SX_CREATE_ALWAYS 2u
#define SX_OPEN_ALWAYS 4u
#define SX_FILE_ATTRIBUTE_NORMAL 0x80u
#define SX_INFINITE 0xffffffffu
#define SX_STD_INPUT_HANDLE ((DWORD) - 10)
#define SX_STD_OUTPUT_HANDLE ((DWORD) - 11)
#define SX_STD_ERROR_HANDLE ((DWORD) - 12)
#define SX_MEM_COMMIT 0x1000u
#define SX_MEM_RESERVE 0x2000u
#define SX_MEM_RELEASE 0x8000u
#define SX_PAGE_READWRITE 0x04u
#define SX_CP_UTF8 65001u

typedef struct {
  DWORD dwLowDateTime;
  DWORD dwHighDateTime;
} SX_FILETIME;
typedef struct {
  WORD wYear, wMonth, wDayOfWeek, wDay, wHour, wMinute, wSecond, wMilliseconds;
} SX_SYSTEMTIME;
typedef struct {
  DWORD dwFileAttributes;
  SX_FILETIME ftCreationTime;
  SX_FILETIME ftLastAccessTime;
  SX_FILETIME ftLastWriteTime;
  DWORD nFileSizeHigh;
  DWORD nFileSizeLow;
  DWORD dwReserved0;
  DWORD dwReserved1;
  char cFileName[260];
  char cAlternateFileName[14];
} SX_WIN32_FIND_DATAA;
typedef struct {
  DWORD cb;
  char *lpReserved, *lpDesktop, *lpTitle;
  DWORD dwX, dwY, dwXSize, dwYSize, dwXCountChars, dwYCountChars,
      dwFillAttribute, dwFlags;
  WORD wShowWindow, cbReserved2;
  BYTE *lpReserved2;
  HANDLE hStdInput, hStdOutput, hStdError;
} SX_STARTUPINFOA;
typedef struct {
  HANDLE hProcess, hThread;
  DWORD dwProcessId, dwThreadId;
} SX_PROCESS_INFORMATION;
typedef struct {
  DWORD nLength;
  void *lpSecurityDescriptor;
  BOOL bInheritHandle;
} SX_SECURITY_ATTRIBUTES;
typedef struct {
  DWORD dwLowDateTime;
  DWORD dwHighDateTime;
} SX_FILETIME2;

__declspec(dllimport) HANDLE __stdcall GetStdHandle(DWORD);
__declspec(dllimport) BOOL __stdcall WriteFile(HANDLE, const void *, DWORD,
                                               DWORD *, void *);
__declspec(dllimport) BOOL __stdcall ReadFile(HANDLE, void *, DWORD, DWORD *,
                                              void *);
__declspec(dllimport) BOOL __stdcall CloseHandle(HANDLE);
__declspec(dllimport) HANDLE __stdcall CreateFileA(const char *, DWORD, DWORD,
                                                   void *, DWORD, DWORD,
                                                   HANDLE);
__declspec(dllimport) DWORD __stdcall SetFilePointer(HANDLE, LONG, LONG *,
                                                     DWORD);
__declspec(dllimport) BOOL __stdcall SetFilePointerEx(HANDLE, LONGLONG,
                                                      LONGLONG *, DWORD);
__declspec(dllimport) BOOL __stdcall GetFileSizeEx(HANDLE, LONGLONG *);
__declspec(dllimport) void __stdcall ExitProcess(UINT);
__declspec(dllimport) DWORD __stdcall GetCurrentProcessId(void);
__declspec(dllimport) DWORD __stdcall GetCurrentThreadId(void);
__declspec(dllimport) HANDLE __stdcall GetCurrentProcess(void);
__declspec(dllimport) void __stdcall GetSystemTimeAsFileTime(SX_FILETIME *);
__declspec(dllimport) void __stdcall GetLocalTime(SX_SYSTEMTIME *);
__declspec(dllimport) void __stdcall Sleep(DWORD);
__declspec(dllimport) HANDLE __stdcall GetProcessHeap(void);
__declspec(dllimport) void *__stdcall HeapAlloc(HANDLE, DWORD, SIZE_T);
__declspec(dllimport) BOOL __stdcall HeapFree(HANDLE, DWORD, void *);
__declspec(dllimport) void *__stdcall VirtualAlloc(void *, SIZE_T, DWORD,
                                                   DWORD);
__declspec(dllimport) BOOL __stdcall VirtualFree(void *, SIZE_T, DWORD);
__declspec(dllimport) DWORD __stdcall GetEnvironmentVariableA(const char *,
                                                              char *, DWORD);
__declspec(dllimport) DWORD __stdcall GetCurrentDirectoryA(DWORD, char *);
__declspec(dllimport) BOOL __stdcall SetCurrentDirectoryA(const char *);
__declspec(dllimport) DWORD __stdcall GetFullPathNameA(const char *, DWORD,
                                                       char *, char **);
__declspec(dllimport) DWORD __stdcall GetFileAttributesA(const char *);
__declspec(dllimport) BOOL __stdcall CreateDirectoryA(const char *, void *);
__declspec(dllimport) BOOL __stdcall RemoveDirectoryA(const char *);
__declspec(dllimport) BOOL __stdcall DeleteFileA(const char *);
__declspec(dllimport) BOOL __stdcall MoveFileA(const char *, const char *);
__declspec(dllimport) BOOL __stdcall CopyFileA(const char *, const char *,
                                               BOOL);
__declspec(dllimport) HANDLE __stdcall FindFirstFileA(const char *,
                                                      SX_WIN32_FIND_DATAA *);
__declspec(dllimport) BOOL __stdcall FindNextFileA(HANDLE,
                                                   SX_WIN32_FIND_DATAA *);
__declspec(dllimport) BOOL __stdcall FindClose(HANDLE);
__declspec(dllimport) BOOL __stdcall CreateProcessA(const char *, char *,
                                                    void *, void *, BOOL, DWORD,
                                                    void *, const char *,
                                                    SX_STARTUPINFOA *,
                                                    SX_PROCESS_INFORMATION *);
__declspec(dllimport) BOOL __stdcall CreatePipe(HANDLE *, HANDLE *, void *,
                                                DWORD);
__declspec(dllimport) BOOL __stdcall SetHandleInformation(HANDLE, DWORD, DWORD);
__declspec(dllimport) DWORD __stdcall WaitForSingleObject(HANDLE, DWORD);
__declspec(dllimport) BOOL __stdcall GetExitCodeProcess(HANDLE, DWORD *);
__declspec(dllimport) HANDLE __stdcall CreateThread(void *, SIZE_T,
                                                    DWORD(__stdcall *)(void *),
                                                    void *, DWORD, DWORD *);
__declspec(dllimport) void __stdcall InitializeCriticalSection(void *);
__declspec(dllimport) void __stdcall DeleteCriticalSection(void *);
__declspec(dllimport) void __stdcall EnterCriticalSection(void *);
__declspec(dllimport) void __stdcall LeaveCriticalSection(void *);
__declspec(dllimport) BOOL __stdcall SwitchToThread(void);
__declspec(dllimport) int __stdcall WideCharToMultiByte(UINT, DWORD,
                                                        const unsigned short *,
                                                        int, char *, int,
                                                        const char *, BOOL *);
__declspec(dllimport) unsigned short *__stdcall GetCommandLineW(void);
__declspec(dllimport) void *__stdcall CommandLineToArgvW(const unsigned short *,
                                                         int *);
__declspec(dllimport) void *__stdcall LocalFree(void *);
__declspec(dllimport) DWORD __stdcall GetModuleFileNameA(HMODULE, char *,
                                                         DWORD);
__declspec(dllimport) int __stdcall WSAGetLastError(void);

typedef void *SOCKET;
typedef struct {
  unsigned short sa_family;
  char sa_data[14];
} SX_SOCKADDR;
typedef struct {
  unsigned short wVersion;
  unsigned short wHighVersion;
  char szDescription[257];
  char szSystemStatus[129];
  unsigned short iMaxSockets;
  unsigned short iMaxUdpDg;
  char *lpVendorInfo;
} SX_WSADATA;
__declspec(dllimport) int __stdcall WSAStartup(unsigned short, SX_WSADATA *);
__declspec(dllimport) int __stdcall WSACleanup(void);
#define SX_WS(name) sx_ws2_##name
#define SX_WSA(name) __attribute__((dllimport)) name
__attribute__((dllimport))
SOCKET __stdcall SX_WS(socket)(int, int, int) __asm__("socket");
__attribute__((dllimport)) int __stdcall SX_WS(bind)(SOCKET,
                                                     const SX_SOCKADDR *,
                                                     int) __asm__("bind");
__attribute__((dllimport)) int __stdcall SX_WS(listen)(SOCKET,
                                                       int) __asm__("listen");
__attribute__((dllimport))
SOCKET __stdcall SX_WS(accept)(SOCKET, SX_SOCKADDR *, int *) __asm__("accept");
__attribute__((dllimport)) int __stdcall SX_WS(connect)(SOCKET,
                                                        const SX_SOCKADDR *,
                                                        int) __asm__("connect");
__attribute__((dllimport)) int __stdcall SX_WS(send)(SOCKET, const char *, int,
                                                     int) __asm__("send");
__attribute__((dllimport)) int __stdcall SX_WS(recv)(SOCKET, char *, int,
                                                     int) __asm__("recv");
__attribute__((dllimport)) int __stdcall SX_WS(sendto)(SOCKET, const char *,
                                                       int, int,
                                                       const SX_SOCKADDR *,
                                                       int) __asm__("sendto");
__attribute__((dllimport)) int __stdcall SX_WS(recvfrom)(
    SOCKET, char *, int, int, SX_SOCKADDR *, int *) __asm__("recvfrom");
__attribute__((dllimport)) int __stdcall SX_WS(setsockopt)(
    SOCKET, int, int, const char *, int) __asm__("setsockopt");
__attribute__((dllimport)) int __stdcall SX_WS(closesocket)(SOCKET) __asm__(
    "closesocket");
__declspec(dllimport) unsigned short __stdcall htons(unsigned short);
__declspec(dllimport) unsigned short __stdcall ntohs(unsigned short);
__declspec(dllimport) int __stdcall WSAPoll(void *, unsigned long, int);
__declspec(dllimport) int __stdcall ioctlsocket(SOCKET, long, unsigned long *);
__declspec(dllimport) int __stdcall BCryptGenRandom(void *, unsigned char *,
                                                    unsigned long,
                                                    unsigned long);

typedef struct {
  int fd;
  unsigned char *buf;
  sx_size_t cap;
  sx_size_t used;
} SX_WIN_HEAP;
static SX_WIN_HEAP sx_wheap;
static int sx_heap_ready = 0;

static void sx_heap_init(void) {
  if (!sx_heap_ready) {
    sx_wheap.cap = 1024 * 1024 * 8;
    sx_wheap.buf = (unsigned char *)VirtualAlloc(SX_NULL, sx_wheap.cap,
                                                 SX_MEM_COMMIT | SX_MEM_RESERVE,
                                                 SX_PAGE_READWRITE);
    sx_wheap.used = 0;
    sx_heap_ready = 1;
  }
}

static void *sx_win_alloc_raw(sx_size_t n) {
  sx_heap_init();
  n = (n + 15u) & ~(sx_size_t)15u;
  if (!sx_wheap.buf || sx_wheap.used + n > sx_wheap.cap) {
    return VirtualAlloc(SX_NULL, n, SX_MEM_COMMIT | SX_MEM_RESERVE,
                        SX_PAGE_READWRITE);
  }
  void *p = sx_wheap.buf + sx_wheap.used;
  sx_wheap.used += n;
  return p;
}

void *malloc(sx_size_t n) {
  sx_size_t *p = (sx_size_t *)sx_win_alloc_raw(n + 16);
  if (!p)
    return SX_NULL;
  p[0] = n;
  return p + 2;
}
void *calloc(sx_size_t count, sx_size_t size) {
  sx_size_t n = count * size;
  void *p = malloc(n);
  if (p)
    memset(p, 0, n);
  return p;
}
void free(void *ptr) { (void)ptr; }
void *realloc(void *ptr, sx_size_t n) {
  if (!ptr)
    return malloc(n);
  sx_size_t *h = ((sx_size_t *)ptr) - 2;
  sx_size_t old = h[0];
  void *p = malloc(n);
  if (!p)
    return SX_NULL;
  memcpy(p, ptr, old < n ? old : n);
  return p;
}

typedef struct sx_win_file {
  HANDLE h;
  HANDLE proc;
  int eof;
  int ungot;
  int is_std;
} FILE;

static FILE sx_win_stdin = {SX_INVALID_HANDLE, 0, -1, 1};
static FILE sx_win_stdout = {SX_INVALID_HANDLE, 0, -1, 1};
static FILE sx_win_stderr = {SX_INVALID_HANDLE, 0, -1, 1};
FILE *stdin = &sx_win_stdin;
FILE *stdout = &sx_win_stdout;
FILE *stderr = &sx_win_stderr;

static void sx_win_init_std(void) {
  if (sx_win_stdin.h == SX_INVALID_HANDLE)
    sx_win_stdin.h = GetStdHandle(SX_STD_INPUT_HANDLE);
  if (sx_win_stdout.h == SX_INVALID_HANDLE)
    sx_win_stdout.h = GetStdHandle(SX_STD_OUTPUT_HANDLE);
  if (sx_win_stderr.h == SX_INVALID_HANDLE)
    sx_win_stderr.h = GetStdHandle(SX_STD_ERROR_HANDLE);
}

FILE *fdopen(int fd, const char *mode) {
  (void)mode;
  sx_win_init_std();
  FILE *f = (FILE *)calloc(1, sizeof(FILE));
  if (!f)
    return SX_NULL;
  f->h = (HANDLE)(sx_intptr_t)fd;
  f->ungot = -1;
  if (fd == 0)
    f->h = sx_win_stdin.h;
  else if (fd == 1)
    f->h = sx_win_stdout.h;
  else if (fd == 2)
    f->h = sx_win_stderr.h;
  return f;
}

FILE *fopen(const char *path, const char *mode) {
  DWORD access = SX_GENERIC_READ;
  DWORD disp = SX_OPEN_EXISTING;
  if (!mode)
    mode = "r";
  if (mode[0] == 'w') {
    access = SX_GENERIC_WRITE;
    disp = SX_CREATE_ALWAYS;
  } else if (mode[0] == 'a') {
    access = SX_GENERIC_WRITE;
    disp = SX_OPEN_ALWAYS;
  } else if (mode[0] == 'r' && mode[1] == '+')
    access = SX_GENERIC_READ | SX_GENERIC_WRITE;
  else if (mode[0] == 'w' && mode[1] == '+') {
    access = SX_GENERIC_READ | SX_GENERIC_WRITE;
    disp = SX_CREATE_ALWAYS;
  } else if (mode[0] == 'a' && mode[1] == '+') {
    access = SX_GENERIC_READ | SX_GENERIC_WRITE;
    disp = SX_OPEN_ALWAYS;
  }
  HANDLE h = CreateFileA(path, access, SX_FILE_SHARE_READ | SX_FILE_SHARE_WRITE,
                         SX_NULL, disp, SX_FILE_ATTRIBUTE_NORMAL, SX_NULL);
  if (h == SX_INVALID_HANDLE)
    return SX_NULL;
  FILE *f = (FILE *)calloc(1, sizeof(FILE));
  if (!f) {
    CloseHandle(h);
    return SX_NULL;
  }
  f->h = h;
  f->ungot = -1;
  if (mode[0] == 'a')
    SetFilePointerEx(h, 0, SX_NULL, 2);
  return f;
}
int fclose(FILE *f) {
  if (!f)
    return -1;
  if (!f->is_std) {
    CloseHandle(f->h);
    free(f);
  }
  return 0;
}
sx_size_t fread(void *ptr, sx_size_t size, sx_size_t nmemb, FILE *f) {
  sx_win_init_std();
  if (!f || !ptr)
    return 0;
  sx_size_t total = size * nmemb;
  DWORD got = 0;
  if (f->ungot >= 0 && total > 0) {
    ((unsigned char *)ptr)[0] = (unsigned char)f->ungot;
    f->ungot = -1;
    return size ? 1 : 0;
  }
  if (!ReadFile(f->h, ptr, (DWORD)total, &got, SX_NULL))
    got = 0;
  if (got == 0)
    f->eof = 1;
  return size ? (sx_size_t)got / size : 0;
}
sx_size_t fwrite(const void *ptr, sx_size_t size, sx_size_t nmemb, FILE *f) {
  sx_win_init_std();
  if (!f || !ptr)
    return 0;
  sx_size_t total = size * nmemb;
  DWORD wrote = 0;
  if (!WriteFile(f->h, ptr, (DWORD)total, &wrote, SX_NULL))
    return 0;
  return size ? (sx_size_t)wrote / size : 0;
}
int fseek(FILE *f, long offset, int whence) {
  if (!f)
    return -1;
  DWORD method = whence == 0 ? 0 : (whence == 1 ? 1 : 2);
  LONGLONG r = 0;
  if (!SetFilePointerEx(f->h, (LONGLONG)offset, &r, method))
    return -1;
  f->eof = 0;
  return 0;
}
long ftell(FILE *f) {
  if (!f)
    return -1;
  LONGLONG r = 0;
  if (!SetFilePointerEx(f->h, 0, &r, 1))
    return -1;
  return (long)r;
}
void rewind(FILE *f) {
  if (f)
    fseek(f, 0, 0);
}
int feof(FILE *f) { return f ? f->eof : 0; }
int fflush(FILE *f) {
  (void)f;
  return 0;
}
int fgetc(FILE *f) {
  sx_win_init_std();
  unsigned char c;
  if (!f)
    return -1;
  if (f->ungot >= 0) {
    int r = f->ungot;
    f->ungot = -1;
    return r;
  }
  if (!ReadFile(f->h, &c, 1, (DWORD[]){0}, SX_NULL)) {
    f->eof = 1;
    return -1;
  }
  return (int)c;
}
int getchar(void) { return fgetc(stdin); }
int fputc(int c, FILE *f) {
  sx_win_init_std();
  unsigned char ch = (unsigned char)c;
  if (!f || !WriteFile(f->h, &ch, 1, (DWORD[]){0}, SX_NULL))
    return -1;
  return ch;
}
int putchar(int c) { return fputc(c, stdout); }
int ungetc(int c, FILE *f) {
  if (!f)
    return -1;
  f->ungot = c & 0xff;
  return c;
}
char *fgets(char *buf, int n, FILE *f) {
  if (!buf || n <= 0 || !f)
    return SX_NULL;
  int i = 0;
  while (i < n - 1) {
    int c = fgetc(f);
    if (c < 0)
      break;
    buf[i++] = (char)c;
    if (c == '\n')
      break;
  }
  if (i == 0)
    return SX_NULL;
  buf[i] = 0;
  return buf;
}

static void sx_win_write(HANDLE h, const char *buf, sx_size_t n) {
  DWORD wrote = 0;
  while (n > 0) {
    DWORD chunk = n > 0x7fffffff ? 0x7fffffff : (DWORD)n;
    if (!WriteFile(h, buf, chunk, &wrote, SX_NULL) || wrote == 0)
      break;
    buf += wrote;
    n -= wrote;
  }
}

int printf(const char *fmt, ...) {
  char buf[4096];
  sx_va_list ap;
  __builtin_va_start(ap, fmt);
  int n = sx_vformat(buf, sizeof(buf), fmt, ap);
  __builtin_va_end(ap);
  sx_win_init_std();
  if (n > 0)
    sx_win_write(sx_win_stdout.h, buf,
                 (sx_size_t)n < sizeof(buf) ? (sx_size_t)n : sizeof(buf) - 1);
  return n;
}
int fprintf(FILE *f, const char *fmt, ...) {
  char buf[4096];
  sx_va_list ap;
  __builtin_va_start(ap, fmt);
  int n = sx_vformat(buf, sizeof(buf), fmt, ap);
  __builtin_va_end(ap);
  if (n > 0 && f)
    sx_win_write(f->h, buf,
                 (sx_size_t)n < sizeof(buf) ? (sx_size_t)n : sizeof(buf) - 1);
  return n;
}
int dprintf(int fd, const char *fmt, ...) {
  char buf[4096];
  sx_va_list ap;
  __builtin_va_start(ap, fmt);
  int n = sx_vformat(buf, sizeof(buf), fmt, ap);
  __builtin_va_end(ap);
  sx_win_init_std();
  if (n > 0) {
    HANDLE h = fd == 2 ? sx_win_stderr.h
                       : (fd == 0 ? sx_win_stdin.h : sx_win_stdout.h);
    sx_win_write(h, buf,
                 (sx_size_t)n < sizeof(buf) ? (sx_size_t)n : sizeof(buf) - 1);
  }
  return n;
}
int puts(const char *s) {
  if (!s)
    s = "(null)";
  sx_win_init_std();
  sx_win_write(sx_win_stdout.h, s, strlen(s));
  sx_win_write(sx_win_stdout.h, "\n", 1);
  return (int)strlen(s) + 1;
}
int fputs(const char *s, FILE *f) {
  if (!s || !f)
    return -1;
  sx_win_write(f->h, s, strlen(s));
  return (int)strlen(s);
}
sx_ssize_t read(int fd, void *buf, sx_size_t n) {
  sx_win_init_std();
  HANDLE h = fd == 0 ? sx_win_stdin.h : (HANDLE)(sx_intptr_t)fd;
  DWORD got = 0;
  if (!ReadFile(h, buf, (DWORD)n, &got, SX_NULL))
    return -1;
  return (sx_ssize_t)got;
}
sx_ssize_t write(int fd, const void *buf, sx_size_t n) {
  sx_win_init_std();
  HANDLE h = fd == 1 ? sx_win_stdout.h
                     : (fd == 2 ? sx_win_stderr.h : (HANDLE)(sx_intptr_t)fd);
  DWORD wrote = 0;
  if (!WriteFile(h, buf, (DWORD)n, &wrote, SX_NULL))
    return -1;
  return (sx_ssize_t)wrote;
}
int close(int fd) {
  if (fd <= 2)
    return 0;
  return CloseHandle((HANDLE)(sx_intptr_t)fd) ? 0 : -1;
}
void exit(int code) { ExitProcess((UINT)code); }
void _exit(int code) { ExitProcess((UINT)code); }
void perror(const char *s) {
  if (s)
    dprintf(2, "%s: error\n", s);
  else
    dprintf(2, "error\n");
}

int _mkdir(const char *path) {
  return CreateDirectoryA(path, SX_NULL) ? 0 : -1;
}
int _rmdir(const char *path) { return RemoveDirectoryA(path) ? 0 : -1; }
int _unlink(const char *path) { return DeleteFileA(path) ? 0 : -1; }
typedef struct sx_win_dir {
  HANDLE h;
  SX_WIN32_FIND_DATAA data;
  int first;
  struct {
    sx_i32 d_ino;
    sx_u16 d_reclen;
    sx_u16 d_namlen;
    char d_name[260];
  } ent;
} DIR;

DIR *opendir(const char *path) {
  char pattern[1024];
  sx_size_t n = strlen(path);
  if (n + 3 >= sizeof(pattern))
    return SX_NULL;
  strcpy(pattern, path);
  if (n > 0 && pattern[n - 1] != '\\' && pattern[n - 1] != '/') {
    pattern[n] = '\\';
    pattern[n + 1] = '*';
    pattern[n + 2] = 0;
  } else {
    pattern[n] = '*';
    pattern[n + 1] = 0;
  }
  DIR *d = (DIR *)calloc(1, sizeof(DIR));
  if (!d)
    return SX_NULL;
  d->h = FindFirstFileA(pattern, &d->data);
  if (d->h == SX_INVALID_HANDLE) {
    free(d);
    return SX_NULL;
  }
  d->first = 1;
  return d;
}
void *readdir(DIR *d) {
  if (!d)
    return SX_NULL;
  if (!d->first) {
    if (!FindNextFileA(d->h, &d->data))
      return SX_NULL;
  }
  d->first = 0;
  d->ent.d_ino = 0;
  d->ent.d_reclen = (sx_u16)sizeof(d->ent);
  d->ent.d_namlen = (sx_u16)strlen(d->data.cFileName);
  strncpy(d->ent.d_name, d->data.cFileName, 259);
  d->ent.d_name[259] = 0;
  return &d->ent;
}
int closedir(DIR *d) {
  if (!d)
    return -1;
  FindClose(d->h);
  free(d);
  return 0;
}

typedef struct {
  sx_i64 tv_sec;
  sx_i64 tv_usec;
} sx_timeval;
typedef struct {
  sx_i32 tm_sec, tm_min, tm_hour, tm_mday, tm_mon, tm_year, tm_wday, tm_yday,
      tm_isdst;
} sx_tm;
static sx_tm sx_tm_storage;
int pipe(int fds[2]) {
  (void)fds;
  return -1;
}
int dup2(int oldfd, int newfd) {
  (void)oldfd;
  (void)newfd;
  return -1;
}
int dup(int fd) {
  (void)fd;
  return -1;
}
int execve(const char *path, char *const argv[], char *const envp[]) {
  (void)path;
  (void)argv;
  (void)envp;
  return -1;
}
int waitpid(int pid, int *status, int options) {
  (void)pid;
  (void)status;
  (void)options;
  return -1;
}
int system(const char *cmd) {
  SX_STARTUPINFOA si;
  SX_PROCESS_INFORMATION pi;
  memset(&si, 0, sizeof(si));
  si.cb = sizeof(si);
  memset(&pi, 0, sizeof(pi));
  if (!CreateProcessA(SX_NULL, (char *)cmd, SX_NULL, SX_NULL, 1, 0, SX_NULL,
                      SX_NULL, &si, &pi))
    return -1;
  WaitForSingleObject(pi.hProcess, SX_INFINITE);
  DWORD code = 0;
  GetExitCodeProcess(pi.hProcess, &code);
  CloseHandle(pi.hProcess);
  CloseHandle(pi.hThread);
  return (int)code;
}
FILE *popen(const char *cmd, const char *mode) {
  if (!cmd)
    return SX_NULL;
  if (mode && mode[0] == 'w')
    return SX_NULL;
  HANDLE rd = SX_INVALID_HANDLE;
  HANDLE wr = SX_INVALID_HANDLE;
  if (!CreatePipe(&rd, &wr, SX_NULL, 0))
    return SX_NULL;
  SetHandleInformation(wr, 1, 0); /* HANDLE_FLAG_INHERIT */
  SX_STARTUPINFOA si;
  SX_PROCESS_INFORMATION pi;
  memset(&si, 0, sizeof(si));
  si.cb = sizeof(si);
  si.dwFlags = 0x00000100; /* STARTF_USESTDHANDLES */
  si.hStdOutput = wr;
  si.hStdError = wr;
  si.hStdInput = GetStdHandle(SX_STD_INPUT_HANDLE);
  memset(&pi, 0, sizeof(pi));
  char cmdline[4096];
  strncpy(cmdline, cmd, sizeof(cmdline) - 1);
  cmdline[sizeof(cmdline) - 1] = 0;
  if (!CreateProcessA(SX_NULL, cmdline, SX_NULL, SX_NULL, 1, 0, SX_NULL,
                      SX_NULL, &si, &pi)) {
    CloseHandle(rd);
    CloseHandle(wr);
    return SX_NULL;
  }
  CloseHandle(wr);
  CloseHandle(pi.hThread);
  FILE *f = (FILE *)calloc(1, sizeof(FILE));
  if (!f) {
    CloseHandle(rd);
    CloseHandle(pi.hProcess);
    return SX_NULL;
  }
  f->h = rd;
  f->proc = pi.hProcess;
  f->ungot = -1;
  return f;
}
int pclose(FILE *f) {
  if (!f)
    return -1;
  CloseHandle(f->h);
  DWORD code = 0;
  if (f->proc) {
    WaitForSingleObject(f->proc, SX_INFINITE);
    GetExitCodeProcess(f->proc, &code);
    CloseHandle(f->proc);
  }
  free(f);
  return (int)code;
}

void (*signal(int sig, void (*handler)(int)))(int) {
  (void)sig;
  return handler;
}
int raise(int sig) {
  (void)sig;
  return -1;
}

struct sx_utsname {
  char sysname[65];
  char nodename[65];
  char release[65];
  char version[65];
  char machine[65];
  char domainname[65];
};
typedef unsigned sx_socklen;
int socket(int d, int t, int p) {
  static int started = 0;
  if (!started) {
    SX_WSADATA w;
    WSAStartup(0x0202, &w);
    started = 1;
  }
  return (int)(sx_intptr_t)SX_WS(socket)(d, t, p);
}
int bind(int fd, const void *addr, sx_socklen len) {
  return SX_WS(bind)((SOCKET)(sx_intptr_t)fd, (const SX_SOCKADDR *)addr,
                     (int)len);
}
int listen(int fd, int backlog) {
  return SX_WS(listen)((SOCKET)(sx_intptr_t)fd, backlog);
}
int accept(int fd, void *addr, sx_socklen *len) {
  return (int)(sx_intptr_t)SX_WS(accept)((SOCKET)(sx_intptr_t)fd,
                                         (SX_SOCKADDR *)addr, (int *)len);
}
int connect(int fd, const void *addr, sx_socklen len) {
  return SX_WS(connect)((SOCKET)(sx_intptr_t)fd, (const SX_SOCKADDR *)addr,
                        (int)len);
}
sx_ssize_t send(int fd, const void *buf, sx_size_t len, int flags) {
  return SX_WS(send)((SOCKET)(sx_intptr_t)fd, (const char *)buf, (int)len,
                     flags);
}
sx_ssize_t recv(int fd, void *buf, sx_size_t len, int flags) {
  return SX_WS(recv)((SOCKET)(sx_intptr_t)fd, (char *)buf, (int)len, flags);
}
int sendto(int fd, const void *buf, sx_size_t len, int flags, const void *addr,
           sx_socklen alen) {
  return SX_WS(sendto)((SOCKET)(sx_intptr_t)fd, (const char *)buf, (int)len,
                       flags, (const SX_SOCKADDR *)addr, (int)alen);
}
int recvfrom(int fd, void *buf, sx_size_t len, int flags, void *addr,
             sx_socklen *alen) {
  return SX_WS(recvfrom)((SOCKET)(sx_intptr_t)fd, (char *)buf, (int)len, flags,
                         (SX_SOCKADDR *)addr, (int *)alen);
}
int setsockopt(int fd, int level, int opt, const void *val, sx_socklen len) {
  return SX_WS(setsockopt)((SOCKET)(sx_intptr_t)fd, level, opt,
                           (const char *)val, (int)len);
}
int poll(void *fds, unsigned long n, int timeout) {
  return WSAPoll(fds, n, timeout);
}
int closesocket(int fd) { return SX_WS(closesocket)((SOCKET)(sx_intptr_t)fd); }
int getsockopt(int fd, int level, int opt, void *val, sx_socklen *len) {
  (void)fd;
  (void)level;
  (void)opt;
  (void)val;
  (void)len;
  return -1;
}
int fcntl(int fd, int cmd, ...) {
  (void)fd;
  (void)cmd;
  return 0;
}
int ferror(FILE *f) {
  (void)f;
  return 0;
}
int clearerr(FILE *f) {
  if (f)
    f->eof = 0;
  return 0;
}
int fileno(FILE *f) { return f ? (int)(sx_intptr_t)f->h : -1; }
char *strdup(const char *s) {
  if (!s)
    return SX_NULL;
  sx_size_t n = strlen(s) + 1;
  char *p = (char *)malloc(n);
  if (p)
    memcpy(p, s, n);
  return p;
}
char *strtok_r(char *s, const char *delim, char **saveptr) {
  if (!delim || !saveptr)
    return SX_NULL;
  if (!s)
    s = *saveptr;
  if (!s)
    return SX_NULL;
  while (*s && strchr(delim, *s))
    s++;
  if (!*s) {
    *saveptr = s;
    return SX_NULL;
  }
  char *start = s;
  while (*s && !strchr(delim, *s))
    s++;
  if (*s) {
    *s = 0;
    s++;
  }
  *saveptr = s;
  return start;
}
static int sx_errno_storage = 0;
int *__errno_location(void) { return &sx_errno_storage; }
int sigaction(int sig, const void *act, void *old) {
  (void)sig;
  (void)act;
  (void)old;
  return -1;
}
int sigemptyset(void *set) {
  if (set)
    memset(set, 0, 128);
  return 0;
}
int sigaddset(void *set, int sig) {
  (void)set;
  (void)sig;
  return 0;
}
int inet_pton(int af, const char *src, void *dst) {
  (void)af;
  (void)src;
  (void)dst;
  return 0;
}
const char *inet_ntop(int af, const void *src, char *dst, sx_size_t size) {
  (void)af;
  (void)src;
  (void)dst;
  (void)size;
  return SX_NULL;
}
void __stack_chk_fail(void) { exit(127); }
int posix_spawn_file_actions_addtcsetpgrp_np(void *a, int fd) {
  (void)a;
  (void)fd;
  return -1;
}
int posix_spawn_file_actions_addclosefrom_np(void *a, int fd) {
  (void)a;
  (void)fd;
  return -1;
}

static char **sx_win_argv = SX_NULL;
static int sx_win_argc = 0;
static char **sx_win_envp = SX_NULL;
char **environ = SX_NULL;

int main(int argc, char **argv);

int _fltused = 0;

static int sx_utf16_to_utf8(const unsigned short *w, char *out, int cap) {
  return WideCharToMultiByte(SX_CP_UTF8, 0, w, -1, out, cap, SX_NULL, SX_NULL);
}

void mainCRTStartup(void) {
  int argc = 0;
  unsigned short *cmd = GetCommandLineW();
  unsigned short **wargv = (unsigned short **)CommandLineToArgvW(cmd, &argc);
  char **argv = (char **)calloc((sx_size_t)argc + 1, sizeof(char *));
  if (wargv && argv) {
    for (int i = 0; i < argc; i++) {
      int n = sx_utf16_to_utf8(wargv[i], SX_NULL, 0);
      if (n <= 0)
        n = 1;
      argv[i] = (char *)malloc((sx_size_t)n + 1);
      if (argv[i])
        sx_utf16_to_utf8(wargv[i], argv[i], n + 1);
    }
    LocalFree(wargv);
  }
  if (!argv) {
    ExitProcess(1);
  }
  sx_win_argc = argc;
  sx_win_argv = argv;
  int code = main(argc, argv);
  ExitProcess((UINT)code);
}

#endif /* platform */
/* Exposed to generated code and the Spectre runtime. */
void __sx_panic_init(void) {}

char **__sx_get_argv(void) {
#if defined(SX_LINUX)
  return sx_argv;
#else
  return sx_win_argv;
#endif
}
int __sx_get_argc(void) {
#if defined(SX_LINUX)
  return sx_argc;
#else
  return sx_win_argc;
#endif
}

/* ------------------------------------------------------------------------- */
/* Stable __sx_* ABI aliases                                                 */
/* ------------------------------------------------------------------------- */

void *__sx_alloc(sx_size_t n) { return malloc(n); }
void *__sx_calloc(sx_size_t count, sx_size_t n) { return calloc(count, n); }
void *__sx_realloc(void *p, sx_size_t n) { return realloc(p, n); }
void __sx_free(void *p) { free(p); }
void *__sx_memcpy(void *dst, const void *src, sx_size_t n) {
  return memcpy(dst, src, n);
}
void *__sx_memmove(void *dst, const void *src, sx_size_t n) {
  return memmove(dst, src, n);
}
void *__sx_memset(void *dst, int v, sx_size_t n) { return memset(dst, v, n); }
int __sx_memcmp(const void *a, const void *b, sx_size_t n) {
  return memcmp(a, b, n);
}
sx_size_t __sx_strlen(const char *s) { return strlen(s); }
int __sx_strcmp(const char *a, const char *b) { return strcmp(a, b); }
char *__sx_strstr(const char *a, const char *b) { return strstr(a, b); }
sx_ssize_t __sx_read(int fd, void *buf, sx_size_t n) {
  return read(fd, buf, n);
}
sx_ssize_t __sx_write(int fd, const void *buf, sx_size_t n) {
  return write(fd, buf, n);
}
int __sx_close(int fd) { return close(fd); }
void __sx_exit(int code) { exit(code); }
sx_i64 __sx_clock(void) { return __sx_sys_clock_us() / 1000000; }
void __sx_sleep(unsigned millis) {
  __sx_sys_nanosleep_us((sx_i64)millis * 1000);
}
void __sx_random_fill(void *buf, sx_size_t n) { __sx_sys_getrandom(buf, n); }

/* ------------------------------------------------------------------------- */
/* Public `sx_*` runtime ABI used by std/runtime                             */
/* ------------------------------------------------------------------------- */

int sx_close(int fd) { return close(fd); }
int sx_fcntl(int fd, int cmd, int arg) { return fcntl(fd, cmd, arg); }

int sx_printf(const char *fmt, ...) {
  char buf[4096];
  sx_va_list ap;
  __builtin_va_start(ap, fmt);
  int n = sx_vformat(buf, sizeof(buf), fmt, ap);
  __builtin_va_end(ap);
  if (n > 0)
    write(1, buf, (sx_size_t)n < sizeof(buf) ? (sx_size_t)n : sizeof(buf) - 1);
  return n;
}
int sx_dprintf(int fd, const char *fmt, ...) {
  char buf[4096];
  sx_va_list ap;
  __builtin_va_start(ap, fmt);
  int n = sx_vformat(buf, sizeof(buf), fmt, ap);
  __builtin_va_end(ap);
  if (n > 0)
    write(fd, buf, (sx_size_t)n < sizeof(buf) ? (sx_size_t)n : sizeof(buf) - 1);
  return n;
}
int sx_puts(const char *s) { return puts(s); }

int sx_system(const char *cmd) { return system(cmd); }
void *sx_popen(const char *cmd, const char *mode) { return popen(cmd, mode); }
int sx_pclose(void *f) { return pclose((FILE *)f); }

int sx_socket(int d, int t, int p) { return socket(d, t, p); }
int sx_bind(int fd, const void *a, unsigned len) { return bind(fd, a, len); }
int sx_listen(int fd, int backlog) { return listen(fd, backlog); }
int sx_accept(int fd, void *a, unsigned *len) { return accept(fd, a, len); }
int sx_connect(int fd, const void *a, unsigned len) {
  return connect(fd, a, len);
}
sx_ssize_t sx_send(int fd, const void *b, sx_size_t n, int flags) {
  return send(fd, b, n, flags);
}
sx_ssize_t sx_recv(int fd, void *b, sx_size_t n, int flags) {
  return recv(fd, b, n, flags);
}
int sx_sendto(int fd, const void *b, sx_size_t n, int flags, const void *a,
              unsigned len) {
  return sendto(fd, b, n, flags, a, len);
}
int sx_recvfrom(int fd, void *b, sx_size_t n, int flags, void *a,
                unsigned *len) {
  return recvfrom(fd, b, n, flags, a, len);
}
int sx_setsockopt(int fd, int level, int opt, const void *v, unsigned len) {
  return setsockopt(fd, level, opt, v, len);
}
int sx_getsockopt(int fd, int level, int opt, void *v, unsigned *len) {
  return getsockopt(fd, level, opt, v, len);
}
int sx_poll(void *fds, unsigned long n, int timeout) {
  return poll(fds, n, timeout);
}
unsigned short sx_htons(unsigned short v) { return htons(v); }
unsigned short sx_ntohs(unsigned short v) { return ntohs(v); }
#if defined(SX_LINUX)
int sx_closesocket(int fd) { return close(fd); }
#else
int sx_closesocket(int fd) { return closesocket(fd); }
#endif

void (*sx_signal(int sig, void (*handler)(int)))(int) {
  return signal(sig, handler);
}
int sx_raise(int sig) { return raise(sig); }

/* ------------------------------------------------------------------------- */
/* Raw syscall boundary used by the Spectre runtime                          */
/* ------------------------------------------------------------------------- */

#if defined(SX_LINUX)
void *__sx_sys_mmap(sx_size_t n) { return sx_map(n); }
void __sx_sys_munmap(void *p, sx_size_t n) {
  sx_sc2(SX_SYS_munmap, (sx_i64)(sx_uintptr_t)p, (sx_i64)n);
}
sx_i64 __sx_sys_read(sx_i64 fd, void *buf, sx_size_t n) {
  return sx_read_fd((int)fd, buf, n);
}
sx_i64 __sx_sys_write(sx_i64 fd, const void *buf, sx_size_t n) {
  return sx_write_fd((int)fd, buf, n);
}
sx_i64 __sx_sys_close(sx_i64 fd) { return sx_close_fd((int)fd); }
sx_i64 __sx_sys_lseek(sx_i64 fd, sx_i64 off, sx_i64 whence) {
  return sx_sc3(SX_SYS_lseek, fd, off, whence);
}
sx_i64 __sx_sys_clock_us(void) {
  sx_timespec ts;
  sx_sc2(SX_SYS_clock_gettime, 0, (sx_i64)(sx_uintptr_t)&ts);
  return ts.tv_sec * 1000000 + ts.tv_nsec / 1000;
}
sx_i64 __sx_sys_nanosleep_us(sx_i64 us) {
  sx_timespec ts;
  ts.tv_sec = us / 1000000;
  ts.tv_nsec = (us % 1000000) * 1000;
  return sx_sc2(SX_SYS_nanosleep, (sx_i64)(sx_uintptr_t)&ts, 0);
}
sx_i64 __sx_sys_getrandom(void *buf, sx_size_t n) {
  return sx_sc3(SX_SYS_getrandom, (sx_i64)(sx_uintptr_t)buf, (sx_i64)n, 0);
}
sx_i64 __sx_sys_futex(sx_i64 addr, sx_i64 op, sx_i64 val, sx_i64 timeout) {
  return sx_sc6(SX_SYS_futex, addr, op, val, timeout, 0, 0);
}
sx_i64 __sx_sys_clone_thread(sx_i64 flags, void *stack, void *entry,
                             void *arg) {
  return __sx_clone_thread(flags, stack, entry, arg);
}
sx_i64 __sx_sys_clone_process(void) { return sx_clone_process(); }
sx_i64 __sx_sys_pipe2(sx_i64 *fds) {
  return sx_sc2(SX_SYS_pipe2, (sx_i64)(sx_uintptr_t)fds, 0);
}
sx_i64 __sx_sys_dup2(sx_i64 oldfd, sx_i64 newfd) {
  return sx_sc2(SX_SYS_dup2, oldfd, newfd);
}
sx_i64 __sx_sys_execve(const char *path, void *argv, void *envp) {
  return sx_sc3(SX_SYS_execve, (sx_i64)(sx_uintptr_t)path,
                (sx_i64)(sx_uintptr_t)argv, (sx_i64)(sx_uintptr_t)envp);
}
sx_i64 __sx_sys_wait4(sx_i64 pid, sx_i64 *status) {
  return sx_sc4(SX_SYS_wait4, pid, (sx_i64)(sx_uintptr_t)status, 0, 0);
}
#else
void *__sx_sys_mmap(sx_size_t n) {
  return VirtualAlloc(SX_NULL, n, SX_MEM_COMMIT | SX_MEM_RESERVE,
                      SX_PAGE_READWRITE);
}
void __sx_sys_munmap(void *p, sx_size_t n) {
  (void)n;
  VirtualFree(p, 0, SX_MEM_RELEASE);
}
sx_i64 __sx_sys_read(sx_i64 fd, void *buf, sx_size_t n) {
  DWORD got = 0;
  if (!ReadFile((HANDLE)(sx_intptr_t)fd, buf, (DWORD)n, &got, SX_NULL))
    return -1;
  return (sx_i64)got;
}
sx_i64 __sx_sys_write(sx_i64 fd, const void *buf, sx_size_t n) {
  DWORD wrote = 0;
  if (!WriteFile((HANDLE)(sx_intptr_t)fd, buf, (DWORD)n, &wrote, SX_NULL))
    return -1;
  return (sx_i64)wrote;
}
sx_i64 __sx_sys_close(sx_i64 fd) {
  return CloseHandle((HANDLE)(sx_intptr_t)fd) ? 0 : -1;
}
sx_i64 __sx_sys_lseek(sx_i64 fd, sx_i64 off, sx_i64 whence) {
  LONGLONG r = 0;
  DWORD method = whence == 0 ? 0 : (whence == 1 ? 1 : 2);
  if (!SetFilePointerEx((HANDLE)(sx_intptr_t)fd, off, &r, method))
    return -1;
  return r;
}
sx_i64 __sx_sys_clock_us(void) {
  SX_FILETIME ft;
  GetSystemTimeAsFileTime(&ft);
  sx_u64 u = ((sx_u64)ft.dwHighDateTime << 32) | ft.dwLowDateTime;
  u = (u - 116444736000000000ull) / 10ull;
  return (sx_i64)u;
}
sx_i64 __sx_sys_nanosleep_us(sx_i64 us) {
  Sleep((DWORD)(us / 1000));
  return 0;
}
sx_i64 __sx_sys_getrandom(void *buf, sx_size_t n) {
  if (BCryptGenRandom(SX_NULL, (unsigned char *)buf, (unsigned long)n, 2) == 0)
    return (sx_i64)n;
  return -1;
}
sx_i64 __sx_sys_futex(sx_i64 addr, sx_i64 op, sx_i64 val, sx_i64 timeout) {
  (void)addr;
  (void)op;
  (void)val;
  (void)timeout;
  return -1;
}
sx_i64 __sx_sys_clone_thread(sx_i64 flags, void *stack, void *entry,
                             void *arg) {
  (void)flags;
  (void)stack;
  (void)entry;
  (void)arg;
  return -1;
}
sx_i64 __sx_sys_clone_process(void) { return -1; }
sx_i64 __sx_sys_pipe2(sx_i64 *fds) {
  (void)fds;
  return -1;
}
sx_i64 __sx_sys_dup2(sx_i64 oldfd, sx_i64 newfd) {
  (void)oldfd;
  (void)newfd;
  return -1;
}
sx_i64 __sx_sys_execve(const char *path, void *argv, void *envp) {
  (void)path;
  (void)argv;
  (void)envp;
  return -1;
}
sx_i64 __sx_sys_wait4(sx_i64 pid, sx_i64 *status) {
  (void)pid;
  (void)status;
  return -1;
}
#endif

/* Extended Spectre-runtime syscall boundary. */
#if defined(SX_LINUX)
sx_i64 __sx_sys_open(const char *path, sx_i64 flags, sx_i64 mode) {
  int f = 0;
  if ((flags & 3) == 0)
    f = SX_O_RDONLY;
  else if ((flags & 3) == 1)
    f = SX_O_WRONLY;
  else
    f = SX_O_RDWR;
  if ((flags & 4) != 0)
    f |= SX_O_CREAT;
  if ((flags & 8) != 0)
    f |= SX_O_TRUNC;
  if ((flags & 16) != 0)
    f |= SX_O_APPEND;
  return sx_open_path(path, f, (int)mode);
}
sx_i64 __sx_sys_stdio(sx_i64 which) { return which; }
sx_i64 __sx_sys_gettid(void) {
#if defined(__x86_64__)
  return sx_sc0(186);
#else
  return sx_sc0(178);
#endif
}
sx_i64 __sx_sys_sched_yield(void) {
#if defined(__x86_64__)
  return sx_sc0(24);
#else
  return sx_sc0(124);
#endif
}
#else
sx_i64 __sx_sys_open(const char *path, sx_i64 flags, sx_i64 mode) {
  DWORD access = SX_GENERIC_READ;
  DWORD disp = SX_OPEN_EXISTING;
  if ((flags & 3) == 1)
    access = SX_GENERIC_WRITE;
  if ((flags & 3) == 2)
    access = SX_GENERIC_READ | SX_GENERIC_WRITE;
  if ((flags & 4) != 0)
    disp = SX_OPEN_ALWAYS;
  if ((flags & 8) != 0)
    disp = SX_CREATE_ALWAYS;
  (void)mode;
  HANDLE h = CreateFileA(path, access, SX_FILE_SHARE_READ | SX_FILE_SHARE_WRITE,
                         SX_NULL, disp, SX_FILE_ATTRIBUTE_NORMAL, SX_NULL);
  if (h == SX_INVALID_HANDLE)
    return -1;
  if ((flags & 16) != 0)
    SetFilePointerEx(h, 0, SX_NULL, 2);
  return (sx_i64)(sx_intptr_t)h;
}
sx_i64 __sx_sys_stdio(sx_i64 which) {
  DWORD id = which == 0
                 ? SX_STD_INPUT_HANDLE
                 : (which == 1 ? SX_STD_OUTPUT_HANDLE : SX_STD_ERROR_HANDLE);
  return (sx_i64)(sx_intptr_t)GetStdHandle(id);
}
sx_i64 __sx_sys_gettid(void) { return (sx_i64)GetCurrentThreadId(); }
sx_i64 __sx_sys_sched_yield(void) { return SwitchToThread() ? 0 : -1; }
sx_i64 __sx_sys_create_thread(void *entry, void *arg) {
  DWORD id = 0;
  HANDLE h =
      CreateThread(SX_NULL, 0, (DWORD(__stdcall *)(void *))entry, arg, 0, &id);
  return h ? (sx_i64)(sx_intptr_t)h : -1;
}
sx_i64 __sx_sys_wait_thread(sx_i64 h) {
  WaitForSingleObject((HANDLE)(sx_intptr_t)h, SX_INFINITE);
  CloseHandle((HANDLE)(sx_intptr_t)h);
  return 0;
}
sx_i64 __sx_sys_detach_thread(sx_i64 h) {
  CloseHandle((HANDLE)(sx_intptr_t)h);
  return 0;
}
#endif

/* ------------------------------------------------------------------------- */
/* Environment / filesystem metadata boundary used by the Spectre runtime   */
/* ------------------------------------------------------------------------- */
#if defined(SX_LINUX)
sx_i64 __sx_sys_getenv(const char *name, char *out, sx_size_t cap) {
  if (!name || !out || !sx_envp)
    return -1;
  sx_size_t n = strlen(name);
  for (char **e = sx_envp; *e; e++) {
    if (strncmp(*e, name, n) == 0 && (*e)[n] == '=') {
      const char *value = *e + n + 1;
      sx_size_t len = strlen(value);
      if (len + 1 > cap)
        return -1;
      memcpy(out, value, len + 1);
      return (sx_i64)len;
    }
  }
  return -1;
}
sx_i64 __sx_sys_getcwd(char *buf, sx_size_t cap) {
  return sx_sc2(SX_SYS_getcwd, (sx_i64)(sx_uintptr_t)buf, (sx_i64)cap);
}
sx_i64 __sx_sys_chdir(const char *path) {
  return sx_sc1(SX_SYS_chdir, (sx_i64)(sx_uintptr_t)path);
}
sx_i64 __sx_sys_access(const char *path, sx_i64 mode) {
  return sx_sc4(SX_SYS_faccessat, SX_AT_FDCWD, (sx_i64)(sx_uintptr_t)path, mode,
                0);
}
sx_i64 __sx_sys_mkdir(const char *path, sx_i64 mode) {
  return sx_sc3(SX_SYS_mkdirat, SX_AT_FDCWD, (sx_i64)(sx_uintptr_t)path, mode);
}
sx_i64 __sx_sys_rmdir(const char *path) {
  return sx_sc3(SX_SYS_unlinkat, SX_AT_FDCWD, (sx_i64)(sx_uintptr_t)path,
                0x200);
}
sx_i64 __sx_sys_remove(const char *path) {
  return sx_sc3(SX_SYS_unlinkat, SX_AT_FDCWD, (sx_i64)(sx_uintptr_t)path, 0);
}
sx_i64 __sx_sys_rename(const char *oldpath, const char *newpath) {
  return sx_sc4(SX_SYS_renameat, SX_AT_FDCWD, (sx_i64)(sx_uintptr_t)oldpath,
                SX_AT_FDCWD, (sx_i64)(sx_uintptr_t)newpath);
}
sx_i64 __sx_sys_readlink(const char *path, char *buf, sx_size_t cap) {
  return sx_sc4(SX_SYS_readlinkat, SX_AT_FDCWD, (sx_i64)(sx_uintptr_t)path,
                (sx_i64)(sx_uintptr_t)buf, (sx_i64)cap);
}
sx_i64 __sx_sys_isatty(sx_i64 fd) {
  unsigned char termios[64];
  return sx_sc3(SX_SYS_ioctl, fd, 0x5401, (sx_i64)(sx_uintptr_t)termios) == 0;
}
sx_i64 __sx_sys_realpath(const char *path, char *out, sx_size_t cap) {
  if (!path || !out || cap == 0)
    return -1;
  char tmp[4096];
  if (path[0] == '/') {
    strncpy(tmp, path, sizeof(tmp) - 1);
    tmp[sizeof(tmp) - 1] = 0;
  } else {
    sx_i64 r =
        sx_sc2(SX_SYS_getcwd, (sx_i64)(sx_uintptr_t)tmp, (sx_i64)sizeof(tmp));
    if (r < 0)
      return -1;
    sx_size_t n = strlen(tmp);
    if (n + 1 + strlen(path) >= sizeof(tmp))
      return -1;
    tmp[n] = '/';
    strcpy(tmp + n + 1, path);
  }
  sx_size_t n = strlen(tmp);
  if (n + 1 > cap)
    return -1;
  memcpy(out, tmp, n + 1);
  return (sx_i64)n;
}
sx_i64 __sx_sys_stat(const char *path, void *buf) {
  unsigned char local[512];
  sx_i64 r = sx_sc4(SX_SYS_newfstatat, SX_AT_FDCWD, (sx_i64)(sx_uintptr_t)path,
                    (sx_i64)(sx_uintptr_t)local, 0);
  if (r < 0)
    return -1;
  sx_fill_compact_stat(local, buf);
  return 0;
}
sx_i64 __sx_sys_fstat(sx_i64 fd, void *buf) {
  unsigned char local[512];
  sx_i64 r =
      sx_sc4(SX_SYS_newfstatat, fd, 0, (sx_i64)(sx_uintptr_t)local, 0x1000);
  if (r < 0)
    return -1;
  sx_fill_compact_stat(local, buf);
  return 0;
}
void *__sx_sys_opendir(const char *path) { return opendir(path); }
void *__sx_sys_readdir(void *d) { return readdir((DIR *)d); }
sx_i64 __sx_sys_closedir(void *d) { return closedir((DIR *)d); }
void __sx_sys_exit(sx_i64 code) { exit((int)code); }
sx_i64 __sx_sys_uname(void *u) {
  return sx_sc1(SX_SYS_uname, (sx_i64)(sx_uintptr_t)u);
}
#else
sx_i64 __sx_sys_getenv(const char *name, char *out, sx_size_t cap) {
  if (!name || !out || cap == 0)
    return -1;
  DWORD n = GetEnvironmentVariableA(name, out, (DWORD)cap);
  if (n == 0 || n >= cap)
    return -1;
  return (sx_i64)n;
}
sx_i64 __sx_sys_getcwd(char *buf, sx_size_t cap) {
  if (!buf || cap == 0)
    return -1;
  DWORD n = GetCurrentDirectoryA((DWORD)cap, buf);
  if (n == 0 || n >= cap)
    return -1;
  return 0;
}
sx_i64 __sx_sys_chdir(const char *path) {
  return SetCurrentDirectoryA(path) ? 0 : -1;
}
sx_i64 __sx_sys_access(const char *path, sx_i64 mode) {
  (void)mode;
  DWORD a = GetFileAttributesA(path);
  return a == 0xffffffffu ? -1 : 0;
}
sx_i64 __sx_sys_mkdir(const char *path, sx_i64 mode) {
  (void)mode;
  return CreateDirectoryA(path, SX_NULL) ? 0 : -1;
}
sx_i64 __sx_sys_rmdir(const char *path) {
  return RemoveDirectoryA(path) ? 0 : -1;
}
sx_i64 __sx_sys_remove(const char *path) { return DeleteFileA(path) ? 0 : -1; }
sx_i64 __sx_sys_rename(const char *oldpath, const char *newpath) {
  return MoveFileA(oldpath, newpath) ? 0 : -1;
}
sx_i64 __sx_sys_readlink(const char *path, char *buf, sx_size_t cap) {
  (void)path;
  (void)buf;
  (void)cap;
  return -1;
}
sx_i64 __sx_sys_isatty(sx_i64 fd) {
  (void)fd;
  return 0;
}
sx_i64 __sx_sys_realpath(const char *path, char *out, sx_size_t cap) {
  if (!path || !out || cap == 0)
    return -1;
  DWORD n = GetFullPathNameA(path, (DWORD)cap, out, SX_NULL);
  if (n == 0 || n >= cap)
    return -1;
  return (sx_i64)n;
}
sx_i64 __sx_sys_stat(const char *path, void *buf) {
  (void)path;
  (void)buf;
  return -1;
}
sx_i64 __sx_sys_fstat(sx_i64 fd, void *buf) {
  (void)fd;
  (void)buf;
  return -1;
}
void *__sx_sys_opendir(const char *path) { return opendir(path); }
void *__sx_sys_readdir(void *d) { return readdir((DIR *)d); }
sx_i64 __sx_sys_closedir(void *d) { return closedir((DIR *)d); }
void __sx_sys_exit(sx_i64 code) { ExitProcess((UINT)code); }
sx_i64 __sx_sys_uname(void *u) {
  if (!u)
    return -1;
  struct sx_utsname *n = (struct sx_utsname *)u;
  strncpy(n->sysname, "Windows", 64);
  strncpy(n->nodename, "", 64);
  strncpy(n->release, "", 64);
  strncpy(n->version, "", 64);
#if defined(__aarch64__)
  strncpy(n->machine, "aarch64", 64);
#else
  strncpy(n->machine, "x86_64", 64);
#endif
  strncpy(n->domainname, "", 64);
  return 0;
}
#endif

/* ------------------------------------------------------------------------- */
/* libc-shaped compatibility wrappers                                        */
/*                                                                           */
/* Generated code and user extern declarations still expect these names.     */
/* The implementation lives in the Spectre runtime; these are just a buncha  */
/* ABI forwarders.                                                           */
/* ------------------------------------------------------------------------- */
#if defined(SX_WINDOWS)
typedef struct {
  sx_i64 tv_sec;
  sx_i64 tv_nsec;
} sx_timespec;
#endif

extern void *sx_localtime(const sx_i64 *t);
extern int sx_pthread_create(void *thread, void *attr, sx_size_t entry,
                             void *arg);
extern int sx_pthread_join(void *thread, void *retval);
extern int sx_pthread_detach(void *thread);
extern void *sx_pthread_self(void);
extern int sx_mutex_init(void *m);
extern int sx_mutex_destroy(void *m);
extern int sx_mutex_lock(void *m);
extern int sx_mutex_unlock(void *m);

char *getenv(const char *name) {
  static char buf[4096];
  if (__sx_sys_getenv(name, buf, sizeof(buf)) < 0)
    return SX_NULL;
  return buf;
}
char *getcwd(char *buf, sx_size_t size) {
  if (!buf)
    return SX_NULL;
  return __sx_sys_getcwd(buf, size) < 0 ? SX_NULL : buf;
}
int chdir(const char *path) { return (int)__sx_sys_chdir(path); }
int access(const char *path, int mode) {
  return (int)__sx_sys_access(path, mode);
}
int mkdir(const char *path, unsigned mode) {
  return (int)__sx_sys_mkdir(path, mode);
}
int rmdir(const char *path) { return (int)__sx_sys_rmdir(path); }
int remove(const char *path) { return (int)__sx_sys_remove(path); }
int rename(const char *oldpath, const char *newpath) {
  return (int)__sx_sys_rename(oldpath, newpath);
}
int unlink(const char *path) { return (int)__sx_sys_remove(path); }
char *realpath(const char *path, char *resolved) {
  char *out = resolved;
  if (!out) {
    out = (char *)malloc(4096);
    if (!out)
      return SX_NULL;
  }
  return __sx_sys_realpath(path, out, 4096) < 0 ? SX_NULL : out;
}
sx_ssize_t readlink(const char *path, char *buf, sx_size_t n) {
  return (sx_ssize_t)__sx_sys_readlink(path, buf, n);
}
int isatty(int fd) { return (int)__sx_sys_isatty(fd); }
int stat(const char *path, void *buf) { return (int)__sx_sys_stat(path, buf); }
int lstat(const char *path, void *buf) { return (int)__sx_sys_stat(path, buf); }
int fstat(int fd, void *buf) { return (int)__sx_sys_fstat(fd, buf); }
int uname(struct sx_utsname *u) { return (int)__sx_sys_uname(u); }
sx_ssize_t getrandom(void *buf, sx_size_t n, unsigned flags) {
  (void)flags;
  return (sx_ssize_t)__sx_sys_getrandom(buf, n);
}
void arc4random_buf(void *buf, sx_size_t n) {
  (void)__sx_sys_getrandom(buf, n);
}

int gettimeofday(sx_timeval *tv, void *tz) {
  (void)tz;
  sx_i64 us = __sx_sys_clock_us();
  if (tv) {
    tv->tv_sec = us / 1000000;
    tv->tv_usec = us % 1000000;
  }
  return 0;
}
sx_tm *localtime(const sx_i64 *timep) { return (sx_tm *)sx_localtime(timep); }
int usleep(unsigned usec) { return (int)__sx_sys_nanosleep_us((sx_i64)usec); }
int sleep(unsigned seconds) {
  return (int)__sx_sys_nanosleep_us((sx_i64)seconds * 1000000);
}
int nanosleep(const sx_timespec *req, sx_timespec *rem) {
  (void)rem;
  if (!req)
    return -1;
  return (int)__sx_sys_nanosleep_us(req->tv_sec * 1000000 +
                                    req->tv_nsec / 1000);
}
sx_i64 time(sx_i64 *out) {
  sx_i64 sec = __sx_sys_clock_us() / 1000000;
  if (out)
    *out = sec;
  return sec;
}

int pthread_create(sx_u64 *thread, void *attr, void *(*fn)(void *), void *arg) {
  return sx_pthread_create(thread, attr, (sx_size_t)(sx_uintptr_t)fn, arg);
}
int pthread_join(sx_u64 thread, void **retval) {
  return sx_pthread_join((void *)(sx_uintptr_t)thread, retval);
}
int pthread_detach(sx_u64 thread) {
  return sx_pthread_detach((void *)(sx_uintptr_t)thread);
}
sx_u64 pthread_self(void) { return (sx_u64)(sx_uintptr_t)sx_pthread_self(); }
int sched_yield(void) { return (int)__sx_sys_sched_yield(); }
int pthread_mutex_init(void *m, void *attr) {
  (void)attr;
  return sx_mutex_init(m);
}
int pthread_mutex_destroy(void *m) { return sx_mutex_destroy(m); }
int pthread_mutex_lock(void *m) { return sx_mutex_lock(m); }
int pthread_mutex_unlock(void *m) { return sx_mutex_unlock(m); }
