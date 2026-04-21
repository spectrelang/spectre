#include <stdio.h>

int classify(int n) {
    switch (n) {
        case 0:
            return 10;
        case 1:
        case 2:
            return 20;
        default:
            return 30;
    }
}

int main() {
    printf("%d\n", classify(2));
}
