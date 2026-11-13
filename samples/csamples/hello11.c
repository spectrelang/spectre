#include <stdio.h>

int count_chars(const u8* text) {
    int n = 0;

    do {
        n++;
    } while (*text++ != '\0');

    return n;
}

int main() {
    char text[] = "hey";
    printf("%d\n", count_chars(text));
}
