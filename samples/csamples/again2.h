typedef struct sapp_allocator {
  void *(*alloc_fn)(size_t size, void *user_data);
  void (*free_fn)(void *ptr, void *user_data);
  void *user_data;
} sapp_allocator;

int main() { return 0; }