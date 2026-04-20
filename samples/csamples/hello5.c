#include <stdio.h>

int f(int arr[10]) {
    return sizeof(arr);
}

int main() {
    int a[10];

    printf("%zu\n", sizeof(a));
    printf("%d\n", f(a)); 

    return 0;
}
