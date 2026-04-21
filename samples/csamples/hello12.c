#include <stdio.h>

int bump_if_nonzero(int* matchlength, int value) {
    if (value)
        (*matchlength)++;

    if (*matchlength > 3)
        return *matchlength;
    else
        return 0;
}

int main() {
    int len = 3;
    printf("%d\n", bump_if_nonzero(&len, 1));
}
