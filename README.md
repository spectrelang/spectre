# Spectre

This repository contains the compiler for the Spectre Programming Language.

Spectre is a statically typed, contract-based language aiming to offer low-level control in combination with explicit correctness. At the same time, it is not too verbose, in the interest of usability, and strikes a balance between the two to enable explicit, trustworthy code to be written.

The complete documentation can be found at https://spectre-docs.pages.dev.

```spectre
val std = use("std")

pub fn main() void! = {
    val xs: list[ref char] = ["hello", "world", "this", "is", "a", "test"]
    for x in xs {
        std.stdio.print("{s}\n", {x})
    }
}
```

Another example, with a simple demonstration of the trust system and pre/postconditions:

```spectre
val std = use("std")

type Stack = {
    data: mut list[i32]
    len:  mut usize
}

pub fn (Stack) push(s: mut self, vl: i32) void = {
    guarded pre {
        not_full: s.len < trust @capacity(s.data)
    }
    trust @append(s.data, vl)
    s.len = s.len + 1
}

pub fn (Stack) pop(s: mut self) option[i32] = {
    pre { 
		not_empty: s.len > 0
	}
    val top = trust @get(s.data, s.len - 1)
    trust @remove(s.data, s.len - 1)
    s.len = s.len - 1
    return top
}

pub fn (Stack) peek(s: mut self) option[i32] = {
    pre { 
		not_empty: s.len > 0 
	}
    return trust @get(s.data, s.len - 1)
}

pub fn (Stack) print_top(s: mut self) void = {
    guarded pre { 
		has_items: s.len > 0 
	}
    match Stack.peek(s) {
        some v => { trust std.stdio.print("top: {d}\n", {v}) }
        none   => {}
    }
}

pub fn main() void! = {
    val s: mut Stack = {data: [], len: 0}
    @reserve(s.data, 10)
    Stack.push(s, 10)
    Stack.push(s, 20)
    Stack.push(s, 30)
    Stack.print_top(s)
    assert Stack.pop(s) == some 30
    assert Stack.pop(s) == some 20
    assert Stack.pop(s) == some 10
}
```

## License

GPL-3.0-only - (C) Navid M - 2026
