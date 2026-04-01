val std = use("std")

pub fn main() void! = {
    val result = can_fail(true)
    match result {
        some x => {
            std.io.put_any("hooray: {d}", {x})
        }
        none => {
            std.io.print(":(")
        }
    }
}

fn can_fail(should_fail: bool) option[i32]! = {
	if (!should_fail) {
		return some 10
	}
	return none
}
