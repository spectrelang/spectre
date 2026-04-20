#include <stdio.h>
#include <stdlib.h>

typedef struct Node {
    int value;
    struct Node* next;
} Node;

int sum(Node* n) {
    int total = 0;
    while (n != NULL) {
        total += n->value;
        n = n->next;
    }
    return total;
}

Node* make_node(int v, Node* next) {
    Node* n = (Node*)malloc(sizeof(Node));
    n->value = v;
    n->next = next;
    return n;
}

int main() {
    Node* list = make_node(1,
                   make_node(2,
                   make_node(3, NULL)));

    int result = sum(list);

    printf("Sum: %d\n", result);

    free(list->next->next);
    free(list->next);
    free(list);

    return 0;
}
