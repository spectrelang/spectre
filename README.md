## Spectre

This repository contains the compiler for the Spectre Programming Language. Spectre is a statically typed, contract-based language aiming to offer low-level control in combination with explicit correctness. At the same time, it aims to not be too verbose, to the point of unusability, and strikes a balance between the two to enable explicit, trustworthy code to be written.

The complete documentation can be found at https://spectre-docs.pages.dev.

```spectre
val std = use("std")

pub fn main() void! = {
    val xs: list[ref char] = ["hello", "world", "this", "is", "a", "test"]
    for x in xs {
        std.io.print("{s}\n", {x})
    }
}
```

Another example, with a simple demonstration of the trust system and pre/postconditions:

```spectre
val std = use("std")
val some_other_module = use("some_other_module.sx")

type Point = {
	x: mut i32
	y: mut i32
}

fn some_function(some_arg: i32, some_other_arg: usize) void = {
	pre {
		is_bigger_than_ten      : some_arg > 10
		is_bigger_than_twenty   : some_other_arg > 20
	}

	val x: i32 = 10                          // This cannot change.
	val y: i32 = 20                          // Neither can this.
	val z: i32 = 30                          // Or this.

	post {
		x_is_ten : x == 10
		y_is_twe : y == 20
		z_is_thi : z == 30
	}

	std.io.print("{d} {d}", {x, y})
}

pub val some_constant = 1000

fn pure_function() void = {
	trust std.io.put("This is trusted now, and can therefore run in a pure function")
}

fn can_fail(should_fail: bool) option[i32]! = {
	if (should_fail) {
		return some 10
	}
	return none
}

pub fn some_other_function() void! = {
	std.io.put("This is some function...without any preconds, postconds, or invariants. Thus the return type is marked !")
}
```
