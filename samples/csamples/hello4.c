#include <stdio.h>
#include <stdlib.h>

typedef struct {
    int x;
    int y;
} P;

void bump(P* p) {
    p->x += 1;
    p->y += p->x;
}

int main() {
    P* p = malloc(sizeof(P));
    p->x = 10;
    p->y = 20;

    bump(p);

    printf("%d %d\n", p->x, p->y);

    free(p);
}
