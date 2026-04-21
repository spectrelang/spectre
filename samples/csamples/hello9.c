#include <stdio.h>

int f() {
    static int x = 0;
    x++;
    return x;
}

int main() {
    printf("%d\n", f());
    printf("%d\n", f());
}
