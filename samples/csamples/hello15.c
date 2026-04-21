#include <stdio.h>
#include <stdlib.h>

long long fib_memo(int n, long long *memo) {
    if (n < 0) return -1;       
    if (n <= 1) return n;
    if (memo[n] != -1) return memo[n];
    memo[n] = fib_memo(n-1, memo) + fib_memo(n-2, memo);
    return memo[n];
}

long long fib(int n) {
    if (n < 0) return -1;          
    long long *memo = malloc((n+1) * sizeof(long long));
    if (!memo) return -1;          
    for (int i = 0; i <= n; ++i) memo[i] = -1;
    long long result = fib_memo(n, memo);
    free(memo);
    return result;
}

int main(void) {
    int n = 50;                    
    long long result = fib(n);
    if (result >= 0)
        printf("fib(%d) = %lld\n", n, result);
    else
        fprintf(stderr, "Error computing fib(%d)\n", n);
    return 0;
}
