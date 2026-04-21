
#include <stdio.h>
#include <stdlib.h>

static int lower_bound(int *arr, int size, int key) {
    int l = 0, r = size; // search in [l, r)
    while (l < r) {
        int m = l + (r - l) / 2;
        if (arr[m] < key) l = m + 1;
        else r = m;
    }
    return l;
}

int lengthOfLIS(int *nums, int n) {
    if (n <= 0) return 0;

    int *tails = (int*)malloc(n * sizeof(int));
    if (!tails) return 0; // allocation failed

    int size = 0;
    for (int i = 0; i < n; ++i) {
        int x = nums[i];
        int pos = lower_bound(tails, size, x);
        tails[pos] = x;
        if (pos == size) ++size;
    }

    free(tails);
    return size;
}

int main(void) {
    int nums[] = {10, 9, 2, 5, 3, 7, 101, 18};
    int n = sizeof(nums) / sizeof(nums[0]);
    int lis = lengthOfLIS(nums, n);
    printf("Length of LIS = %d\n", lis);
    return 0;
}
