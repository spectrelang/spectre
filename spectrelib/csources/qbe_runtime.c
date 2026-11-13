#if defined(_WIN32)
#include <windows.h>
#endif

#ifdef SX_FREESTANDING
typedef __SIZE_TYPE__ size_t;
#if defined(__TINYC__)
typedef __PTRDIFF_TYPE__ intptr_t;
#else
typedef __INTPTR_TYPE__ intptr_t;
#endif
void *malloc(size_t);
void *calloc(size_t, size_t);
void *realloc(void *, size_t);
void free(void *);
void *memcpy(void *, const void *, size_t);
void *memset(void *, int, size_t);
size_t strlen(const char *);
int snprintf(char *, size_t, const char *, ...);
#else
#include <stdint.h>
#include <stdlib.h>
#include <stdio.h>
#include <string.h>
#endif
typedef long long (*sxqbe_alloc_fn)(long long,unsigned long long);
typedef int (*sxqbe_free_fn)(long long,long long);
typedef long long (*sxqbe_realloc_fn)(long long,long long,unsigned long long);
typedef struct {sxqbe_alloc_fn alloc;sxqbe_free_fn free;sxqbe_realloc_fn realloc;long long ctx;} sxqbe_alloc_desc;
static long long* sxqbe_ok(long long v){long long*r=(long long*)calloc(1,16);if(!r)return 0;r[0]=0;r[1]=v;return r;}
static long long* sxqbe_err(long long e){long long*r=(long long*)calloc(1,16);if(!r)return 0;r[0]=1;r[1]=e;return r;}
static void* sxqbe_alloc_call(sxqbe_alloc_fn f,long long ctx,long long size){long long r=f(ctx,(unsigned long long)size);return r?(void*)(intptr_t)(r-1):0;}
static long long sxqbe_realloc_call(sxqbe_realloc_fn f,long long ctx,void*p,long long size){long long r=f(ctx,(long long)(intptr_t)p,(unsigned long long)size);return r?(long long)(intptr_t)(r-1):0;}
static long long* sxqbe_ok_af(sxqbe_alloc_fn af,long long ctx,long long v){if(!af)return sxqbe_ok(v);long long* r=(long long*)sxqbe_alloc_call(af,ctx,16);if(!r)return 0;r[0]=0;r[1]=v;return r;}
static long long* sxqbe_err_af(sxqbe_alloc_fn af,long long ctx,long long e){if(!af)return sxqbe_err(e);long long* r=(long long*)sxqbe_alloc_call(af,ctx,16);if(!r)return 0;r[0]=1;r[1]=e;return r;}
int sxqbe_list_grow(long long* hdr,long long need){if(!hdr)return 0;long long cap=hdr[2];long long* buf=(long long*)(intptr_t)hdr[0];if(need<=cap)return 1;long long ncap=cap<4?8:cap*2;if(ncap<need)ncap=need;sxqbe_alloc_desc* d=(sxqbe_alloc_desc*)(intptr_t)hdr[3];if(d){long long nb=buf?sxqbe_realloc_call(d->realloc,d->ctx,buf,ncap*8):(long long)(intptr_t)sxqbe_alloc_call(d->alloc,d->ctx,ncap*8);if(!nb)return 0;hdr[0]=nb;hdr[2]=ncap;}else{long long* nb=(long long*)realloc(buf,(size_t)(ncap*8));if(!nb)return 0;hdr[0]=(long long)(intptr_t)nb;hdr[2]=ncap;}return 1;}
long long* sxqbe_list_alloc(void* aft,void* fft,void* rft,long long ctx){sxqbe_alloc_fn af=(sxqbe_alloc_fn)aft;sxqbe_free_fn ff=(sxqbe_free_fn)fft;sxqbe_realloc_fn rf=(sxqbe_realloc_fn)rft;long long* hdr;sxqbe_alloc_desc* d;if(af){hdr=(long long*)sxqbe_alloc_call(af,ctx,32);d=(sxqbe_alloc_desc*)sxqbe_alloc_call(af,ctx,sizeof(sxqbe_alloc_desc));}else{hdr=(long long*)calloc(1,32);d=(sxqbe_alloc_desc*)calloc(1,sizeof(sxqbe_alloc_desc));}if(!hdr||!d){if(af){if(hdr)ff(ctx,(long long)(intptr_t)hdr);if(d)ff(ctx,(long long)(intptr_t)d);}else{if(hdr)free(hdr);if(d)free(d);}return sxqbe_err_af(af,ctx,0);}memset(hdr,0,32);memset(d,0,sizeof(sxqbe_alloc_desc));long long cap=8;long long buf=(long long)(intptr_t)sxqbe_alloc_call(af,ctx,cap*8);if(!buf){if(af){ff(ctx,(long long)(intptr_t)hdr);ff(ctx,(long long)(intptr_t)d);}else{free(hdr);free(d);}return sxqbe_err_af(af,ctx,0);}d->alloc=af;d->free=ff;d->realloc=rf;d->ctx=ctx;hdr[0]=buf;hdr[1]=0;hdr[2]=cap;hdr[3]=(long long)(intptr_t)d;return sxqbe_ok_af(af,ctx,(long long)(intptr_t)hdr);}
void sxqbe_list_free(long long*);
long long* sxqbe_list_from(void* aft,void* fft,void* rft,long long ctx,long long* elems,long long count){long long* r=sxqbe_list_alloc(aft,fft,rft,ctx);if(!r||r[0]!=0)return r;long long* hdr=(long long*)r[1];if(count>0){if(!sxqbe_list_grow(hdr,count)){sxqbe_list_free(hdr);return sxqbe_err_af((sxqbe_alloc_fn)aft,ctx,0);}long long* buf=(long long*)hdr[0];long long i;for(i=0;i<count;i++)buf[i]=elems[i];hdr[1]=count;}return r;}
long long* sxqbe_args_alloc(void* aft,void* fft,void* rft,long long ctx,long long argc,long long argv){sxqbe_alloc_fn af=(sxqbe_alloc_fn)aft;sxqbe_free_fn ff=(sxqbe_free_fn)fft;sxqbe_realloc_fn rf=(sxqbe_realloc_fn)rft;long long* hdr;sxqbe_alloc_desc* d;if(af){hdr=(long long*)sxqbe_alloc_call(af,ctx,32);d=(sxqbe_alloc_desc*)sxqbe_alloc_call(af,ctx,sizeof(sxqbe_alloc_desc));}else{hdr=(long long*)calloc(1,32);d=(sxqbe_alloc_desc*)calloc(1,sizeof(sxqbe_alloc_desc));}if(!hdr||!d){if(af){if(hdr)ff(ctx,(long long)(intptr_t)hdr);if(d)ff(ctx,(long long)(intptr_t)d);}else{if(hdr)free(hdr);if(d)free(d);}return sxqbe_err(0);}long long bytes=argc>0?argc*8:8;long long buf=(long long)(intptr_t)sxqbe_alloc_call(af,ctx,bytes);if(!buf){if(af){ff(ctx,(long long)(intptr_t)hdr);ff(ctx,(long long)(intptr_t)d);}else{free(hdr);free(d);}return sxqbe_err(0);}d->alloc=af;d->free=ff;d->realloc=rf;d->ctx=ctx;hdr[0]=buf;hdr[1]=argc;hdr[2]=argc>0?argc:1;hdr[3]=(long long)(intptr_t)d;char** av=(char**)(intptr_t)argv;long long i;for(i=0;i<argc;i++)((long long*)(intptr_t)buf)[i]=(long long)(intptr_t)av[i];return sxqbe_ok((long long)(intptr_t)hdr);}
long long* sxqbe_fmt_alloc(void* aft,void* fft,void* rft,long long ctx,const char* f,long long* p){sxqbe_alloc_fn af=(sxqbe_alloc_fn)aft;sxqbe_free_fn ff=(sxqbe_free_fn)fft;sxqbe_realloc_fn rf=(sxqbe_realloc_fn)rft;long long cap=4096;char* b=(char*)sxqbe_alloc_call(af,ctx,cap);if(!b)return sxqbe_err(0);int n=p?snprintf(b,(size_t)cap,f,p[0],p[1],p[2],p[3],p[4],p[5],p[6],p[7],p[8],p[9],p[10],p[11],p[12],p[13],p[14],p[15]):snprintf(b,(size_t)cap,"%s",f);if(n<0){ff(ctx,(long long)(intptr_t)b);return sxqbe_err(0);}if((long long)n>=cap){long long ncap=(long long)n+1;long long nb=sxqbe_realloc_call(rf,ctx,b,ncap);if(!nb){ff(ctx,(long long)(intptr_t)b);return sxqbe_err(0);}b=(char*)(intptr_t)nb;cap=ncap;n=p?snprintf(b,(size_t)cap,f,p[0],p[1],p[2],p[3],p[4],p[5],p[6],p[7],p[8],p[9],p[10],p[11],p[12],p[13],p[14],p[15]):snprintf(b,(size_t)cap,"%s",f);if(n<0){ff(ctx,(long long)(intptr_t)b);return sxqbe_err(0);}}return sxqbe_ok((long long)(intptr_t)b);}
char* sxqbe_rewrite_fmt(const char* s){if(!s)return(char*)"";size_t n=strlen(s);int ps=0;for(size_t i=0;i+1<n;i++){char c=s[i+1];if(s[i]=='%'&&(c=='%'||c=='d'||c=='i'||c=='o'||c=='u'||c=='x'||c=='X'||c=='e'||c=='E'||c=='f'||c=='F'||c=='g'||c=='G'||c=='a'||c=='A'||c=='c'||c=='s'||c=='p'||c=='l')){ps=1;break;}}char*o=(char*)malloc(n*4+2);size_t oi=0;if(ps){memcpy(o,s,n);o[n]=0;return o;}for(size_t i=0;i<n;){unsigned char c=(unsigned char)s[i];if(c=='\\'&&i+1<n&&(s[i+1]=='{'||s[i+1]=='}')){o[oi++]=s[i+1];i+=2;}else if(c=='{'){size_t j=i+1;while(j<n&&s[j]!='}')j++;if(j<n){size_t tlen=j-(i+1);int okk=0;char sp[8];memset(sp,0,8);if(tlen==1){char sc=s[i+1];if(sc=='d'||sc=='i'||sc=='s'||sc=='x'||sc=='c'||sc=='p'||sc=='f'){okk=1;sp[0]=sc;}}else if(tlen<=6){memcpy(sp,s+i+1,tlen);if((tlen==2&&((sp[0]=='l'&&(sp[1]=='d'||sp[1]=='u'||sp[1]=='x'))||(sp[0]=='z'&&(sp[1]=='u'||sp[1]=='d'))))||(tlen==3&&sp[0]=='l'&&sp[1]=='l'&&(sp[2]=='d'||sp[2]=='u'||sp[2]=='x')))okk=1;}if(okk){if(tlen==1){char sc=sp[0];if(sc=='d'||sc=='i'){memcpy(o+oi,"%lld",4);oi+=4;}else if(sc=='s'){memcpy(o+oi,"%s",2);oi+=2;}else if(sc=='x'){memcpy(o+oi,"%llx",4);oi+=4;}else if(sc=='c'){memcpy(o+oi,"%",2);oi+=2;}else if(sc=='p'){memcpy(o+oi,"%p",2);oi+=2;}else{memcpy(o+oi,"%f",2);oi+=2;}}else{o[oi++]='%';memcpy(o+oi,sp,tlen);oi+=tlen;}}else{memcpy(o+oi,s+i,j-i+1);oi+=j-i+1;}i=j+1;}else{o[oi++]=c;i++;}}else{o[oi++]=c;i++;}}o[oi]=0;return o;}
void sxqbe_list_free(long long* h){if(!h)return;long long buf=h[0];sxqbe_alloc_desc* d=(sxqbe_alloc_desc*)(intptr_t)h[3];if(d){if(buf){if(d->free)d->free(d->ctx,buf);else free((void*)(intptr_t)buf);}if(d->free)d->free(d->ctx,(long long)(intptr_t)h);else free(h);if(d->free)d->free(d->ctx,(long long)(intptr_t)d);else free(d);}else{if(buf)free((void*)(intptr_t)buf);free(h);}}
long long* sxqbe_list_ensure(long long** slot){if(!slot)return 0;if(!*slot){long long* h=(long long*)calloc(1,32);if(!h)return 0;h[0]=0;h[1]=0;h[2]=0;h[3]=0;*slot=h;}return *slot;}
long long sxqbe_list_len(long long* h){return h?h[1]:0;}
long long sxqbe_list_cap(long long* h){return h?h[2]:0;}
long long sxqbe_list_buf(long long* h){return h?h[0]:0;}
int sxqbe_list_valid(long long* h,long long idx){return h&&idx>=0&&idx<h[1];}
int sxqbe_list_append(long long* h,long long v){if(!h)return 0;if(!sxqbe_list_grow(h,h[1]+1))return 0;((long long*)(intptr_t)h[0])[h[1]++]=v;return 1;}
int sxqbe_list_insert(long long* h,long long idx,long long v){if(!h||idx<0||idx>h[1])return 0;if(!sxqbe_list_grow(h,h[1]+1))return 0;long long* b=(long long*)(intptr_t)h[0];long long i;for(i=h[1];i>idx;i--)b[i]=b[i-1];b[idx]=v;h[1]++;return 1;}
long long sxqbe_list_remove(long long* h,long long idx){if(!h||idx<0||idx>=h[1])return 0;long long* b=(long long*)(intptr_t)h[0];long long v=b[idx];long long i;for(i=idx;i<h[1]-1;i++)b[i]=b[i+1];h[1]--;return v;}
void sxqbe_list_reserve(long long* h,long long cap){if(!h)return;if(cap>h[2])sxqbe_list_grow(h,cap);}
typedef void* (*sxqbe_raw_alloc_fn)(long long,unsigned long long);
static void* sxqbe_raw_alloc_call(sxqbe_raw_alloc_fn f,long long ctx,long long size){return f(ctx,(unsigned long long)size);}
static void* sxqbe_wrap_alloc(void* af,long long ctx,long long size){return sxqbe_raw_alloc_call((sxqbe_raw_alloc_fn)af,ctx,size);}
long long sxqbe_some_alloc(void* af,long long ctx,long long value){void* p=sxqbe_raw_alloc_call((sxqbe_raw_alloc_fn)af,ctx,8);if(!p)return 0;*(long long*)p=value;return (long long)(intptr_t)p+1;}
long long sxqbe_ok_alloc(void* af,long long ctx,long long value){long long* p=(long long*)sxqbe_raw_alloc_call((sxqbe_raw_alloc_fn)af,ctx,16);if(!p)return 0;p[0]=0;p[1]=value;return (long long)(intptr_t)p;}
long long sxqbe_err_alloc(void* af,long long ctx,long long value){long long* p=(long long*)sxqbe_raw_alloc_call((sxqbe_raw_alloc_fn)af,ctx,16);if(!p)return 0;p[0]=1;p[1]=value;return (long long)(intptr_t)p;}
#if defined(_WIN32) && !defined(SX_FREESTANDING)
#include <stdarg.h>
int dprintf(int fd,const char*fmt,...){char buf[4096];va_list ap;va_start(ap,fmt);int n=vsnprintf(buf,sizeof(buf),fmt,ap);va_end(ap);if(n>0){FILE*f=fd==2?stderr:stdout;fwrite(buf,1,(size_t)n,f);fflush(f);}return n;}
#endif
extern void std__allocators__ListArena__alloc(long long*,long long,unsigned long long);
extern int std__allocators__ListArena__free(long long,long long);
extern void std__allocators__ListArena__realloc(long long*,long long,long long,unsigned long long);
long long sx__allocadapt_allocators__ListArena_alloc(long long ctx,unsigned long long size){long long r[3]={0,0,0};std__allocators__ListArena__alloc(r,ctx,size);return (r[0]!=0&&r[1]!=0)?r[1]+1:0;}
int sx__allocadapt_allocators__ListArena_free(long long ctx,long long ptr){std__allocators__ListArena__free(ctx,ptr);return 0;}
long long sx__allocadapt_allocators__ListArena_realloc(long long ctx,long long ptr,unsigned long long size){long long r[3]={0,0,0};std__allocators__ListArena__realloc(r,ctx,ptr,size);return (r[0]!=0&&r[1]!=0)?r[1]+1:0;}
extern void std__allocators__HeapAllocator__alloc(long long*,long long,unsigned long long);
extern int std__allocators__HeapAllocator__free(long long,long long);
extern void std__allocators__HeapAllocator__realloc(long long*,long long,long long,unsigned long long);
long long sx__allocadapt_allocators__HeapAllocator_alloc(long long ctx,unsigned long long size){long long r[3]={0,0,0};std__allocators__HeapAllocator__alloc(r,ctx,size);return (r[0]!=0&&r[1]!=0)?r[1]+1:0;}
int sx__allocadapt_allocators__HeapAllocator_free(long long ctx,long long ptr){std__allocators__HeapAllocator__free(ctx,ptr);return 0;}
long long sx__allocadapt_allocators__HeapAllocator_realloc(long long ctx,long long ptr,unsigned long long size){long long r[3]={0,0,0};std__allocators__HeapAllocator__realloc(r,ctx,ptr,size);return (r[0]!=0&&r[1]!=0)?r[1]+1:0;}
#include "execinfo.h"


#if defined(_WIN32)
#include <windows.h>
#include <dbghelp.h>
extern void sx_panic_report(int, void*, int);
static LONG WINAPI sx__panic_filter(EXCEPTION_POINTERS* info) {
    void* frames[64];
    WORD count = CaptureStackBackTrace(0, 64, frames, 0);
    int code = (int)info->ExceptionRecord->ExceptionCode;
    sx_panic_report(code, frames, (int)count);
    return EXCEPTION_EXECUTE_HANDLER;
}
void sx__panic_shim_init(void) {
    SetUnhandledExceptionFilter(sx__panic_filter);
}
#else
#include <signal.h>
#include <execinfo.h>
extern void sx_panic_report(int, void*, int);
static void sx__panic_trampoline(int sig) {
    signal(sig, SIG_DFL);
    void* frames[64];
    int count = backtrace(frames, 64);
    sx_panic_report(sig, frames, count);
    raise(sig);
}
void sx__panic_shim_init(void) {
    signal(SIGSEGV, sx__panic_trampoline);
    signal(SIGABRT, sx__panic_trampoline);
    signal(SIGBUS, sx__panic_trampoline);
    signal(SIGFPE, sx__panic_trampoline);
    signal(SIGILL, sx__panic_trampoline);
}
#endif
