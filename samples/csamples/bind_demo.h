#ifndef SPECTRE_BIND_DEMO_H
#define SPECTRE_BIND_DEMO_H

#include <stdint.h>

#define BIND_DEMO_VERSION "1.0"
#define BIND_DEMO_LIMIT 64
#define BIND_DEMO_FLAG (1u << 5)
#define BIND_DEMO_FUNCTION_LIKE(x) ((x) + 1)

typedef struct BindDemoVector {
    float x;
    float y;
} BindDemoVector;

typedef enum BindDemoColor {
    BIND_DEMO_RED = 1,
    BIND_DEMO_GREEN,
    BIND_DEMO_BLUE = 7
} BindDemoColor;

typedef void (*BindDemoCallback)(int code, const char *message);

extern int bind_demo_counter;
extern const char *bind_demo_name;
extern BindDemoVector bind_demo_origin;
extern int bind_demo_table[16];

int bind_demo_add(int a, int b);
BindDemoVector bind_demo_vec_add(BindDemoVector a, BindDemoVector b);
void bind_demo_set_callback(BindDemoCallback callback);
long bind_demo_long(long value);

static inline int bind_demo_square(int value) {
    return value * value;
}

#endif
