#include <stdio.h>
#include <stdlib.h>

typedef struct {
    int n;
    int data[];
} Arr;

int main() {
    Arr* a = malloc(sizeof(Arr) + 3 * sizeof(int));
    a->n = 3;

    for (int i = 0; i < 3; i++) {
        a->data[i] = i;
    }

    printf("%d\n", a->data[2]);

    free(a);
}
