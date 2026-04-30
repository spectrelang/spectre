#ifndef M_IMAGE_H
#define M_IMAGE_H

#include <stdint.h>

#define M_IMAGE_VERSION 1

#ifdef __cplusplus
extern "C" {
#endif

#ifndef MIAPI
#define MIAPI extern
#endif

#define M_VOID   0
#define M_BOOL   1
#define M_BYTE   2
#define M_UBYTE  3
#define M_SHORT  4
#define M_USHORT 5
#define M_INT    6
#define M_UINT   7
#define M_HALF   8
#define M_FLOAT  9
#define M_DOUBLE 10

struct m_image
{
   void *data;
   int size;
   int width;
   int height;
   int comp;
   u8 type;
};

#define M_IMAGE_IDENTITY() {0, 0, 0, 0, 0, 0}

int main() {

}
