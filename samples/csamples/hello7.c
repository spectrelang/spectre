#include <stdio.h>

int main() {
    int x = 10;
    int* a = &x;
    int* b = &x;

    *a = 20;

    printf("%d\n", *b);
}
