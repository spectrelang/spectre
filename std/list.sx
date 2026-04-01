// Layout of a List struct (3 fields × 8 bytes each):
//   offset 0: ptr  — ref void, pointer to heap buffer
//   offset 8: len  — usize, number of elements
//   offset 16: cap — usize, allocated capacity in elements
//
// Each element is one pointer-sized slot (8 bytes).
// Values are stored and retrieved as untyped (ref void / usize).

val ELEMENT_SIZE: usize = 8

// Allocate a new List with the given initial capacity.
// Returns a ref void pointing to the List header (caller owns it).
pub fn list_new(initial_cap: usize) ref void! = {
    val header: ref void = @alloc(24)   // 3 fields × 8 bytes
    val buf: ref void = @alloc(initial_cap * ELEMENT_SIZE)
    @memset(buf, 0, initial_cap * ELEMENT_SIZE)
    // store ptr, len=0, cap
    @store(header, buf)
    @store(@ptradd(header, 8), 0)
    @store(@ptradd(header, 16), initial_cap)
    return header
}

// Free a List and its backing buffer.
pub fn list_free(list: ref void) void! = {
    val buf: ref void = @load(list)
    @free(buf)
    @free(list)
}

// Return the number of elements in the list.
pub fn list_len(list: ref void) usize! = {
    return @load(@ptradd(list, 8))
}

// Return the current capacity.
pub fn list_cap(list: ref void) usize! = {
    return @load(@ptradd(list, 16))
}

// Grow the backing buffer to at least new_cap elements.
fn list_grow(list: ref void, new_cap: usize) void! = {
    val old_buf: ref void = @load(list)
    val new_buf: ref void = @realloc(old_buf, new_cap * ELEMENT_SIZE)
    @store(list, new_buf)
    @store(@ptradd(list, 16), new_cap)
}

// Append a value to the end of the list.
pub fn list_push(list: ref void, value: usize) void! = {
    val len: usize = @load(@ptradd(list, 8))
    val cap: usize = @load(@ptradd(list, 16))
    if (len >= cap) {
        val new_cap: usize = cap * 2 + 1
        list_grow(list, new_cap)
    }
    val buf: ref void = @load(list)
    val slot: ref void = @ptradd(buf, len * ELEMENT_SIZE)
    @store(slot, value)
    @store(@ptradd(list, 8), len + 1)
}

// Get the element at index i. Returns none if out of bounds.
pub fn list_get(list: ref void, i: usize) option[usize]! = {
    val len: usize = @load(@ptradd(list, 8))
    if (i >= len) {
        return none
    }
    val buf: ref void = @load(list)
    val slot: ref void = @ptradd(buf, i * ELEMENT_SIZE)
    return some @load(slot)
}

// Set the element at index i. Returns 1 on success, 0 if out of bounds.
pub fn list_set(list: ref void, i: usize, value: usize) usize! = {
    val len: usize = @load(@ptradd(list, 8))
    if (i >= len) {
        return 0
    }
    val buf: ref void = @load(list)
    val slot: ref void = @ptradd(buf, i * ELEMENT_SIZE)
    @store(slot, value)
    return 1
}

// Pop the last element. Returns none if the list is empty.
pub fn list_pop(list: ref void) option[usize]! = {
    val len: usize = @load(@ptradd(list, 8))
    if (len == 0) {
        return none
    }
    val new_len: usize = len - 1
    val buf: ref void = @load(list)
    val slot: ref void = @ptradd(buf, new_len * ELEMENT_SIZE)
    val value: usize = @load(slot)
    @store(@ptradd(list, 8), new_len)
    return some value
}
