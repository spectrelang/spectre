pub fn add(x: i32, y: i32) i32 = {
    return x + y
}

pub fn main() i32 = {
    return 0
}

test {
    assert add(1, 2) == 3
    assert add(1, 2) == 4 // assertion failure
}
