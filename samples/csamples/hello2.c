#include <stdio.h>
#include <stdlib.h>

#define SQUARE(x) ((x) * (x))
#define MAX(a,b) ((a) > (b) ? (a) : (b))

typedef struct {
    int x;
    int y;
} Point;

static int add(int a, int b) {
    return a + b;
}

int apply(int (*fn)(int, int), int a, int b) {
    return fn(a, b);
}

int main() {
    Point p = { .x = 3, .y = 4 };

    int s = SQUARE(p.x + p.y);
    int m = MAX(p.x, p.y);

    int result = apply(add, s, m);

    int *arr = malloc(3 * sizeof(int));
    for (int i = 0; i < 3; i++) {
        arr[i] = result + i;
    }

    printf("Result: %d\n", arr[2]);

    free(arr);
    return 0;
}
