val std = use("std")

type SomeType = {
    x: i32
    y: mut i32
}

pub fn main() void! = {
    val st: mut SomeType = {x: 30, y: 40}
    st.y = 30
    st.x = 10 // This should error. x is immutable.
    std.io.put_any("{d}", {st.y})
}
