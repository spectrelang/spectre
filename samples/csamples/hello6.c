#include <stdio.h>

int main() {
    int i = 1;
    int a = i++ + i++;
    int b = ++i + ++i;

    printf("%d %d %d\n", i, a, b);
}
