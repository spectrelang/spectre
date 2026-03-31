# Language Specification: Spectre (v0.1.0)

Spectre is a systems-level, contract-based programming language designed for formal verification and safety. 

Here, software correctness is a first-class citizen, we achieve this end by integrating pre-conditions, post-conditions, and a manual trust-propagation system.

---

## 1. Core Principles
* Safety by Contract: Functions define their own success criteria via `pre` and `post` blocks.
* Explicit Trust: Functions lacking formal contracts must be explicitly marked as trusted using the `!` suffix.
* Immutability by Default: All bindings use `val` and are immutable unless specifically marked with `mut`.

---

## 2. Syntax and Keywords

### 2.1 Variable Bindings
Variables are declared using the `val` keyword. Mutability must be explicitly opted into at the type level.
* **Immutable**: `val x: i32 = 10`
* **Mutable**: `val x: mut i32 = 10`
* **Mutable Buffer**: `val buf: mut []char = "data"`

### 2.2 Functional Blocks
Functions use an assignment-style syntax with braces.
```rust
fn name(arg: type) ReturnType = {
    // Body
}
```

---

## 3. Contract System

### 3.1 Pre-conditions and Post-conditions
Contracts are defined using `pre` and `post` blocks. These blocks contain labeled boolean expressions.
* **Pre-conditions**: Must evaluate to true before function execution.
* **Post-conditions**: Must evaluate to true before function return.

```rust
fn divide(a: i32, b: i32) i32 = {
    pre {
        not_zero : b != 0
    }
    val result = a / b
    post {
        is_scaled : result <= a
    }
    return result
}
```

We can also have unlabelled boolean expressions, and the boolean expressions can contain function calls and complex expressions.


```rust
fn divide(a: i32, b: i32) i32 = {
    pre {
        b != 0
        is_proper(b)
    }
    val result = a / b
    post {
        result <= a
        is_a_good_result(a)
    }
    return result
}
```

### 3.2 The Trust Marker (`!`)
Functions that do not contain formal contracts (or perform unverifiable side effects like I/O) must append a `!` to their return type. This signifies a "Trusted" but unverified state.
* **Verified**: `fn add(a: i32) i32` (Requires contracts)
* **Trusted**: `fn print_data() void!` (Unverified)

### 3.3 Trust Propagation
Any verified function calling a trusted function (`!`) must either:
1.  Adopt the `!` marker itself, propagating the lack of verification up the call stack.
2.  Use a manual override (e.g., `trust`) to assert safety at the call site and maintain its verified status.

---

## 4. Type System

### 4.1 Primitive Types
Standard signed/unsigned integers (`i32`, `u32`, `usize`), floats, and booleans.

### 4.2 Option Types
Handled via the `option[T]` generic with `some` and `none` variants.
```rust
fn check(fail: bool) option[i32]! = {
    if (fail) { return some 10 }
    return none
}
```

### 4.3 Structs
Composite types use braces. Fields can be individually marked `mut`.
```rust
type Point = {
    x: mut i32
    y: mut i32
}
```

---

## 5. Implementation Strategy
* **File Extension**: `.spr`
* **Lowering**: Contracts are currently lowered to runtime branches (`br_if`). If a condition fails, the program panics to prevent undefined behavior.
* **Future Scope**: Static analysis and range-tracking to elide runtime checks where mathematical proof is possible.