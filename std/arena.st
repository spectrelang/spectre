/// An arena hands out memory from a single large block.
/// Individual allocations cannot be freed — the whole arena is freed at once.
/// This is ideal for request-scoped or phase-scoped allocations.
///
/// Layout of an Arena struct (3 fields × 8 bytes each):
///   offset 0:  buf    — ref void, pointer to the backing block
///   offset 8:  offset — usize, current bump position (bytes used)
///   offset 16: cap    — usize, total capacity in bytes

/// Alignment helper: round up to the nearest 8-byte boundary.
fn align8(n: usize) usize! = {
    return (n + 7) - ((n + 7) - ((n + 7) / 8) * 8)
}

/// Create a new arena backed by a block of `capacity` bytes.
pub fn arena_new(capacity: usize) ref void! = {
    val header: ref void = @alloc(24)
    val buf: ref void = @alloc(capacity)
    @memset(buf, 0, capacity)
    @store(header, buf)
    @store(@ptradd(header, 8), 0)
    @store(@ptradd(header, 16), capacity)
    return header
}

/// Free the entire arena and its backing block.
pub fn arena_free(arena: ref void) void! = {
    val buf: ref void = @load(arena)
    @free(buf)
    @free(arena)
}

/// Allocate `size` bytes from the arena.
/// Returns none if there is not enough space remaining.
pub fn arena_alloc(arena: ref void, size: usize) option[ref void]! = {
    val off: usize = @load(@ptradd(arena, 8))
    val cap: usize = @load(@ptradd(arena, 16))
    val aligned: usize = (size + 7) - ((size + 7) - ((size + 7) / 8) * 8)
    if (off + aligned > cap) {
        return none
    }
    val buf: ref void = @load(arena)
    val ptr: ref void = @ptradd(buf, off)
    @store(@ptradd(arena, 8), off + aligned)
    return some ptr
}

// Return the number of bytes currently used.
pub fn arena_used(arena: ref void) usize! = {
    return @load(@ptradd(arena, 8))
}

// Return the total capacity in bytes.
pub fn arena_cap(arena: ref void) usize! = {
    return @load(@ptradd(arena, 16))
}

// Return the number of bytes remaining.
pub fn arena_remaining(arena: ref void) usize! = {
    val off: usize = @load(@ptradd(arena, 8))
    val cap: usize = @load(@ptradd(arena, 16))
    return cap - off
}

// Reset the arena — all previously allocated pointers become invalid.
pub fn arena_reset(arena: ref void) void! = {
    val buf: ref void = @load(arena)
    val cap: usize = @load(@ptradd(arena, 16))
    @memset(buf, 0, cap)
    @store(@ptradd(arena, 8), 0)
}
