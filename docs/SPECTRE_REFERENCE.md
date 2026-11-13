# The Spectre Programming Language Specification

This document specifies the core Spectre programming language as accepted by the reference compiler and as observed in compiled programs. It is a working specification of the language, not of the standard library. Standard-library types and functions are referenced only where they constitute the canonical realization of a language-level convention, principally the allocator protocol. The specification does not define the QBE intermediate representation, the platform assembler, the linker, or the C ABI, except where those are observable from source programs.

The words _shall_, _must_, _should_, and _may_ are used in their conventional normative sense. A statement marked as an implementation note describes the reference implementation and does not by itself constrain every conforming implementation.

## 1. Scope and Conformance

### 1.1 Scope

Spectre is a statically typed, design-by-contract, low-level systems programming language. Its source files use the extension `.sx`. The core language comprises lexical structure, type syntax and type identity, declarations and modules, expressions, statements, compile-time and runtime contract checking, the trust discipline, raw memory access, allocator-threaded APIs, and the observable execution model.

The language provides no garbage collector, no borrow checker, no lifetime inference, no closures, and no automatic destructors. Memory is obtained and released explicitly. Correctness obligations that are not expressible in the type system are expressed through preconditions, postconditions, type invariants, assertions, and the trust discipline.

### 1.2 Conformance

A conforming implementation shall accept every source unit derivable from the grammar in Section 10 and shall reject every source unit that violates a static rule stated in this document. A source unit that passes lexical, syntactic, and static analysis is a _program_. The behavior of a program is determined by the dynamic rules of this document and by the semantics of the operations it invokes.

Where this document states that a property is implementation-defined, an implementation may choose any behavior consistent with the observable requirements of the language. Where this document states that a property is unspecified, no requirement is imposed. The reference implementation reports diagnostics with a source file and line; the exact wording of a diagnostic is not normative.

The reference implementation performs the static checks described in Sections 3 through 7. It is not a general-purpose theorem prover. A condition that the static checker cannot establish is not rejected solely for that reason; the check is either deferred to runtime or remains an obligation on the programmer, as specified for the construct in question.

### 1.3 Compilation model

A compilation unit is a source file. The compiler resolves imported modules recursively and analyses the resulting module tree as one program. The reference pipeline proceeds from source text to tokens, to an abstract syntax tree, through module resolution and semantic analysis, to QBE intermediate representation, and then to platform-specific assembly which is assembled and linked with the platform C runtime and any requested libraries.

The compiler itself is written in Spectre. The principal implementation files are `src/ast/lexer.sx`, `src/ast/parser.sx`, `src/sema/sema.sx`, `src/module/module.sx`, and `src/codegen/codegen.sx` in the compiler repository. The standard library is located under `spectrelib/` and is imported through paths beginning with `std/`.

## 2. Lexical Structure

### 2.1 Source representation

A source file is a sequence of octets. The reference lexer reads a NUL-terminated byte sequence and interprets the bytes of identifiers, keywords, punctuation, and numeric literals as ASCII. String and character literals denote byte sequences. Line numbering begins at one and column numbering begins at one. A line feed advances the line number and resets the column to one; every other byte advances the column by one. Token positions record the line and column of the first byte of the token.

### 2.2 Whitespace and comments

The whitespace bytes are space (`0x20`), horizontal tab (`0x09`), line feed (`0x0A`), and carriage return (`0x0D`). A comment begins with `//` and extends to the next line feed or to the end of the source. There are no block comments. The byte sequence `/*` is lexed as the division operator followed by the multiplication operator, not as a comment. A `#` byte outside a literal is not a comment introducer and constitutes a lexical error.

A comment of the form `//!suppress "feature"` is a suppression directive. It applies to the next top-level declaration or specification method and suppresses the diagnostic named by `feature` for that declaration. The reference compiler recognizes `"unused-function"`, which suppresses the warning for an otherwise unused private function. A suppression directive is otherwise an ordinary comment; unrecognized feature names have no effect.

Newlines are not significant syntactic tokens. There is no automatic semicolon insertion. Statements are separated by the grammar itself, and a semicolon is not a statement terminator in the language. The lexer produces a semicolon token for the byte `;`, but the parser does not accept it in any production; consequently, a semicolon in source text is a syntax error except where it appears inside a string or character literal.

### 2.3 Reserved words

The reserved words of the language are `fn`, `spec`, `val`, `mut`, `pub`, `type`, `pre`, `post`, `return`, `if`, `else`, `elif`, `trust`, `inline`, `use`, `ref`, `for`, `in`, `some`, `none`, `ok`, `err`, `match`, `true`, `false`, `defer`, `break`, `continue`, `as`, `assert`, `when`, `union`, `enum`, `extern`, `link`, `lib`, `guarded`, `not`, `test`, `guard`, and `invariant`.

The following words are contextual rather than reserved. The words `is`, `or`, and `otherwise` are recognized in specific grammatical positions and are otherwise ordinary identifiers. The word `_` is an ordinary identifier used as a wildcard or inference marker. The words `self`, `void`, `argspack`, `option`, `result`, and `list` are ordinary identifiers whose text receives special interpretation in type position or in a method signature. The word `header` is a modifier in an extern declaration. Intrinsic names following `@` are ordinary identifiers validated by the semantic analyzer rather than by the lexer.

All keywords and contextual words are case-sensitive. A reserved word cannot be used as an identifier.

### 2.4 Identifiers

An identifier begins with an ASCII letter in the range `A` through `Z` or `a` through `z`, or with the underscore byte `_`. Subsequent bytes may be letters, decimal digits, or underscores. Identifiers are case-sensitive and are restricted to ASCII. A single underscore is a valid identifier.

### 2.5 Numeric literals

A decimal integer literal consists of one or more decimal digits. A hexadecimal integer literal begins with the lowercase prefix `0x` and continues with hexadecimal digits. The sign of a numeric literal is not part of the literal; `-` is always a separate prefix operator. There are no digit separators, integer suffixes, binary or octal literals, or hexadecimal floating-point literals.

A floating-point literal consists of a decimal integer part followed by a decimal point, or by an exponent marker `e` or `E`. The digits after the decimal point and after the exponent marker are optional in the lexer. An exponent may carry an explicit `+` or `-` sign. A leading decimal point is not part of a floating-point literal; `.5` is the field-access token followed by the integer literal `5`. A literal such as `1..2` begins with the floating-point token `1.`.

### 2.6 Character literals

A character literal is enclosed in single quotation marks and denotes one byte. The recognized escape sequences are `\n` for line feed (`10`), `\t` for horizontal tab (`9`), `\r` for carriage return (`13`), `\0` for the zero byte, `\'` for a single quotation mark (`39`), and `\\` for a reverse solidus (`92`). A reverse solidus followed by any other byte denotes that byte. Character literals have type `u8` in the reference type inference rules.

### 2.7 String literals

An ordinary string literal is enclosed in double quotation marks and denotes a borrowed `StringView` with a compile-time-known length. The compiler constructs the view directly; importing `std/string` is not required merely to use a string literal. A line feed may occur inside an ordinary string literal and is preserved in the denoted bytes. The recognized escape sequences are `\n` for line feed, `\t` for horizontal tab, `\r` for carriage return, `\"` for a double quotation mark, `\\` for a reverse solidus, and `\xHH` for the byte denoted by the two hexadecimal characters following `\x`. A reverse solidus followed by any other byte is preserved as the reverse solidus and that byte.

A C string literal is written with a `c` prefix, as in `c"hello"`. It denotes a NUL-terminated byte sequence of type `ref u8` and accepts the same escapes as an ordinary string literal. Use `c"..."` when a raw C string is required, for example when calling a C function that expects `ref u8`.

A raw string literal begins with two reverse solidus bytes `\\` and performs no escape processing. The text after the opening marker on the same line is the first line of the value. A following line whose first non-space, non-tab, non-carriage-return bytes are `\\` continues the value; the two marker bytes are removed and a line feed is inserted before the remainder of that line. Indentation before the continuation marker is discarded. The literal terminates at the first following line that does not begin with the continuation marker after optional whitespace, or at the end of the source. Raw string literals denote `StringView` values.

### 2.8 Punctuation and operators

The punctuation and operator tokens are `(`, `)`, `{`, `}`, `[`, `]`, `:`, `;`, `,`, `.`, `...`, `=`, `!`, `@`, `+`, `-`, `*`, `/`, `%`, `++`, `--`, `+=`, `-=`, `==`, `!=`, `<`, `>`, `<=`, `>=`, `&&`, `||`, `&`, `|`, `^`, `<<`, `>>`, `->`, `=>`, `~`, and `?`. The token `->` and the token `;` are produced by the lexer but are not used by any parser production. The token `...` is used only in an extern parameter list. The token `?` is the postfix error-propagation operator. The token `@` introduces an intrinsic.

Multi-character tokens are recognized maximally from left to right. For example, `...` is recognized before `.`, `++` before `+`, `->` before `-`, `==` and `=>` before `=`, `!=` before `!`, `<=` and `<<` before `<`, `>=` and `>>` before `>`, `&&` before `&`, and `||` before `|`.

## 3. Types and Values

### 3.1 Type expressions

A type expression is one of the primitive type names, a qualified named type, a reference type, a mutable type, a slice or fixed array, a function-pointer type, or one of the built-in parameterized types `option`, `result`, and `list`. The complete type grammar is given in Section 10.

The type constructor `ref T` denotes a raw reference to a value of type `T`. The type constructor `mut T` denotes a mutable place or a mutable parameter of type `T`; it may occur in a parameter type, in a field type through the `mut` declaration modifier, or nested within another type expression. The constructors may be nested, so `ref mut T` and `mut ref T` are distinct type expressions. A slice type is written `[]T`, and a fixed array type is written `[N]T` or `[NAME]T`, where the size is a decimal integer literal or a qualified constant name.

The parameterized types are written `option[T]`, `result[T, E]`, and `list[T]`. These three names are recognized by their identifier text in type position and are not user-definable type constructors. A function-pointer type is written `fn(T1, T2, ...) R`; it has no parameter names and no variadic form. The special type `argspack` denotes the heterogeneous argument pack accepted by a variadic formatting or forwarding function. The type `void` denotes the absence of a value.

### 3.2 Primitive types

The primitive types and their reference-implementation sizes are the following. Sizes are expressed in bytes and are observable through `@sizeof`.

| Type    | Size | Interpretation                         |
| ------- | ---: | -------------------------------------- |
| `i8`    |    1 | signed two's-complement integer        |
| `i16`   |    2 | signed two's-complement integer        |
| `i32`   |    4 | signed two's-complement integer        |
| `i64`   |    8 | signed two's-complement integer        |
| `u8`    |    1 | unsigned integer                       |
| `u16`   |    2 | unsigned integer                       |
| `u32`   |    4 | unsigned integer                       |
| `u64`   |    8 | unsigned integer                       |
| `usize` |    8 | unsigned pointer-sized integer         |
| `isize` |    8 | signed pointer-sized integer           |
| `f32`   |    4 | IEEE 754 binary32 floating-point value |
| `f64`   |    8 | IEEE 754 binary64 floating-point value |
| `bool`  |    4 | truth value                            |
| `void`  |    0 | no value                               |

Integer arithmetic is performed at the width of the underlying machine representation. The reference implementation does not insert overflow or underflow checks. Division and remainder by zero are not checked. The right-shift operator is emitted as a machine shift; the language does not provide a separate arithmetic-shift operator. A value used in a condition is false when it is zero and true when it is nonzero.

### 3.3 Reference types and mutability

A value of type `ref T` is a machine address referring to a value of type `T`. The null reference is written `none` and is also the empty value of an option type. References are not automatically dereferenced, and the language provides no implicit conversion from an integer to a reference or from one reference type to another. Assignment to a reference-typed binding requires either the value `none` or an explicit cast to exactly the destination reference type.

Bindings are immutable by default. A local binding is mutable only when its declaration contains the `mut` modifier before the type, as in `val x: mut i64 = 0`. A parameter is a mutable value parameter when its type is written `mut T`, and a mutable reference parameter when its type is written `mut ref T`. A field is mutable only when declared with the `mut` modifier as in `name: mut T`. Mutation through a reference requires the reference binding itself to be mutable; the compiler diagnoses an attempt to assign through an immutable reference.

The reference implementation does not perform lifetime analysis, alias analysis, provenance tracking, or bounds checking for raw references. A reference whose referent has been released is a dangling reference, and dereferencing it is outside the defined behavior of the language.

### 3.4 Named types, fields, and layout

A named type is introduced by a `type` declaration. Its body consists of field declarations and at most one invariant block. Each field has a name and a type, and may be marked mutable. A field that is not marked mutable cannot be assigned after initialization. A type body may be empty. Named types are nominal: two type declarations with identical field sequences are distinct types.

A value of a named type may be constructed with a named struct literal, as in `Point{x: 1, y: 2}`, or with an anonymous struct literal, as in `{x: 1, y: 2}` when the destination type is known. Every field of the destination type shall be present in a named construction. The expression `T{}` is zero initialization of type `T` and is distinct from constructing an empty type. Field initializers are evaluated in source order in the reference implementation.

A named type declared without `extern` is laid out without padding: the offset of a field is the sum of the sizes of the preceding fields, and the size of the type is the sum of its field sizes. A type declared with `extern type` is laid out according to the C ABI of the target platform, including alignment and padding. The `extern` qualifier therefore changes both layout and ABI compatibility.

An invariant block contains one or more boolean expressions separated by whitespace. It constrains the values of the fields of the type. Invariant semantics are specified in Section 7.5.

### 3.5 Tagged unions

A union declaration introduces a tagged union. Each variant has a name and a parenthesized list of payload types. A variant with no payload is written with empty parentheses, as in `None()`. An untagged union is not part of the language. A union value is constructed by naming a variant and supplying its payload values, as in `Circle(radius)` or `Rectangle(width, height)`.

A union value is represented by a tag identifying the active variant and a payload region containing the fields of that variant. Pattern matching on a union selects an arm by tag and binds the payload fields of the selected variant. A match arm for a variant with no payload is written `Variant()`; a match arm for a variant with payloads names the payload bindings after the variant name.

### 3.6 Enumerations

An enumeration declaration introduces a set of bare variant names. Variants have no payload and no explicit discriminant values. The variants are numbered from zero in declaration order. An enumeration value is denoted as `Enum.Variant`. Enumerations are compared and matched by their variant identity.

### 3.7 Option and result types

The type `option[T]` has the values `none` and `some x`, where `x` has type `T`. The type `result[T, E]` has the values `ok x`, `ok()` when the success value is absent, and `err e`, where `x` has type `T` and `e` has type `E`. The constructors `some`, `ok`, and `err` are prefix forms. `none` is a primary form and takes no operand.

An option value is eliminated by a match with `some` and `none` arms, or by the `is` form of an `if` statement. A result value is eliminated by a match with `ok` and `err` arms, by the `is` form of an `if` statement, or by the postfix `?` operator. In the reference representation, `none` is also the null value of a reference type. Option and result values are unboxed: they consist of an 8-byte tag followed by a payload region sized for the largest variant payload.

The postfix operator `?` applies to an expression of result type. If the expression denotes an error, the enclosing function returns immediately with that result value. If the expression denotes success, the operator produces the success payload. A `?` expression used in a function whose declared result type does not match the propagated error is not a well-formed use of the operator.

### 3.8 Lists

The type `list[T]` denotes a dynamically sized sequence of values of type `T`. A list is created by an allocator-backed constructor. The form `@list(allocator, e1, e2, ...)` creates a list from the supplied elements, the form `@list(allocator)` creates an empty list, and the form `@listalloc[A, T](allocator)` creates an empty list whose allocator type is `A`. The element type argument `T` of `@listalloc` may be omitted when the destination type determines it. A bracket expression list `[e1, e2, ...]` constructs a fixed array value and does not construct a list; using it where a list is required is an error.

The reference representation of a list value is an 8-byte handle that refers to a separately allocated 32-byte header. The header contains, in order, a data-buffer reference, the current length, the current capacity, and an optional allocator descriptor. Consequently, `@sizeof(list[T])` is 8, and copying a list copies the handle rather than the header. Each element occupies one 64-bit slot. The core list operations are `@append`, `@insert`, `@remove`, `@get`, `@set`, `@len`, `@reserve`, `@capacity`, and `@listfree`. The operation `@get` returns an option and yields `none` when the index is out of bounds. The operations `@insert`, `@remove`, and `@set` do not perform bounds checking; a caller that requires in-bounds behavior shall establish it through a contract or a guard.

### 3.9 Fixed arrays and slices

A fixed array type `[N]T` denotes `N` consecutive elements of type `T`. A slice type `[]T` denotes a reference to a sequence together with its length; the reference representation uses two 64-bit words. Fixed arrays are laid out consecutively and are packed in non-extern aggregates. A fixed array value may be written as a bracket expression list `[e1, e2, ...]`, whose length shall match the declared array size. The core does not provide an indexing expression. Element access is performed with `@get` for checked access or with pointer arithmetic and a load operation for unchecked access.

### 3.10 Function types

A function-pointer type has the form `fn(T1, T2, ...) R`. It denotes a reference to a function that accepts the listed parameter types and returns a value of type `R`. Function values are obtained by naming a function or by taking the address of a function with `@addr`. The core language has no closures and no nested function declarations. A function value does not carry an environment.

### 3.11 Specifications and implementations

A specification declares an interface. Its body contains function declarations, each optionally marked `pub`, and each having a name, a parameter list, and a return type. A specification describes a set of method signatures; it is not a runtime object and has no vtable representation.

A named type conforms to a specification by writing the specification name in parentheses after `type`, as in `type (SomeSpec) SomeType = { ... }`. The type shall provide a namespaced function whose name and signature match every function declared by the specification. The special parameter type `self` in a specification method denotes the implementing type; it is substituted textually in the method signature. Method calls are resolved statically to namespaced functions. A specification may be empty, in which case it acts as an opaque handle type.

### 3.12 Type identity and conversions

Named types are nominal. Two named types are identical only when they denote the same declaration, modulo module qualification. Specifications are related to their implementing types through the declared conformance relation. The built-in parameterized types are identical when their names and type arguments are identical.

The conversion expression `e as T` requests conversion of `e` to type `T`. Numeric conversions between integer and floating-point representations are permitted and are performed by the target machine. A conversion that changes the width or signedness of an integer does not check that the value is representable. A pointer may be converted to another pointer type or to an integer representation by an explicit cast. The intrinsic `@bitcast(value, T)` performs a representation-level conversion and is subject to the trust rules of Section 7.9.

The intrinsic `@sizeof(T)` yields the size in bytes of type `T` as a compile-time integer. For a named type the result follows the layout rules of Section 3.4. For a reference, option, function pointer, list, result, slice, or fixed array the result follows the representation described in the corresponding subsection. There is no `@alignof` intrinsic in the core language.

## 4. Declarations, Modules, and Names

### 4.1 Modules and compilation units

Each source file is a module. A module contains a sequence of top-level items. Top-level items are declarations of functions, types, enumerations, unions, specifications, external entities, linker directives, tests, and values. The module name is derived from the file path during module resolution; source programs refer to imported modules through the alias bound by a `use` declaration.

Top-level names are registered before the bodies of the module are analysed. This permits a function to refer to a function, type, or global declared later in the same module. Duplicate function definitions are rejected. An unused private function produces a warning; an unused import and an unused local variable are errors. The warning for a particular function can be suppressed with the directive `//!suppress "unused-function"` immediately before its declaration.

```spectre
//!suppress "unused-function"
fn helper() i32 = {
    return 42
}
```

### 4.2 Imports

An import is introduced by a top-level value declaration whose right-hand side is a `use` expression, as in `val stdio = use("std/stdio")`. The import binds the module to the identifier on the left-hand side. The `use` form is not a general expression and may appear only as the entire right-hand side of a top-level value declaration. An import cannot be marked `pub`, and a type annotation on an import is rejected.

A path beginning with `std/` is resolved relative to the standard-library root `spectrelib/`; for example, `std/stdio` denotes `spectrelib/stdio/core.sx`. A path ending in `.sx` is resolved relative to the directory of the importing file. Otherwise the resolver searches for `path/core.sx`, `path.sx`, and `path/path.sx` in the executable directory and the workspace root. Import resolution is recursive. A cyclic import is an error, and an unresolved import is an error.

A module member is accessed through the import alias. A function in another module is called as `alias.function(...)`, and a namespaced function is called as `alias.Type.method(...)`. Only direct members of an imported module are accessible; the alias of a module imported by another module is not re-exported through that module. The reference implementation eliminates an import that is not referenced anywhere in the importing module.

### 4.3 Visibility

The modifier `pub` makes a declaration visible to importing modules. It may precede a function declaration, a type declaration, an enumeration declaration, a union declaration, a specification declaration, a value declaration, or an extern declaration. A call to a function that is not public in its defining module is rejected. The entry point `main` shall be public. A `use` declaration cannot be public.

The modifier `pub` cannot precede a `link` directive, a `lib` directive, a test declaration, or a top-level `when` item. A public value declared in bulk form is accepted syntactically, but the reference implementation does not propagate the public flag through the bulk form; consequently, bulk values should not be relied upon for cross-module visibility.

### 4.4 Top-level values

A top-level immutable value is written `val name = expression` or `val name: T = expression`. It is initialized before the program entry point is called. A top-level mutable value is written `val name: mut T = expression`; its storage is mutable for the duration of the program. The bulk form `val (a = e1, b = e2, ...)` declares several top-level values in one declaration. Commas between bulk entries are optional in the grammar. The bulk form is not permitted inside a function body.

### 4.5 Functions and methods

A function declaration has the form `fn name(param: T, ...) R = { ... }`. The return type is mandatory. A function body may be omitted, in which case the declaration introduces a signature without a definition. When a body is present, the `=` is mandatory and the body is a braced statement block. The modifiers `inline` and `trust` may appear between `=` and the opening brace, in any order, as in `fn f() T = inline { ... }` or `fn f() T = inline trust { ... }`. The `inline` modifier requests that the compiler expand calls to the function at each call site; it is a hint and does not change the observable semantics of the program. The `trust` modifier marks the function as requiring trust at call sites.

A parameter is written `name: T`. A mutable value parameter is written `name: mut T`. A mutable reference parameter is written `name: mut ref T`. A parameter of type `argspack` accepts a brace-enclosed heterogeneous pack. Parameter names are mandatory, commas separate parameters, and a trailing comma is permitted.

A method is a namespaced free function. It is declared with a parenthesized receiver before the function name, as in `fn (Counter) increment(c: mut self) void = { ... }`. The receiver is a type name, not an implicit parameter. The special type name `self` in a parameter or return type is replaced textually by the receiver type. There is no implicit receiver, no dynamic dispatch, and no access to a hidden `this` binding. A method is called as `Counter.increment(c)`.

### 4.6 Generic functions

A function declaration may introduce type parameters in brackets after its name, as in `fn identity[T](value: T) T`. A type parameter may carry a constraint after a colon, as in `fn render[A: Allocator](ac: A) ...`. The constraint names a specification; an explicit type argument satisfies the constraint when the corresponding type declares conformance to that specification.

Generic instantiation is written after the function expression, as in `identity[i64](value)` or `render[allocators.Arena](arena, name)`. The type argument list may contain the wildcard `_`, which requests inference from the argument types. A generic function requires an explicit type-argument list at its call site; the reference implementation does not infer all type arguments solely from the value arguments. Generics are a compile-time mechanism: the reference implementation monomorphizes a generic function for each concrete instantiation. User-defined generic types are not part of the language; only `option`, `result`, and `list` accept type arguments in type position.

### 4.7 Extern declarations and linker directives

An external function is declared with an extern convention, a name, a parameter list, a return type, and a symbol string, as in `extern (C) fn strlen(s: ref u8) i64 = "strlen"`. The convention identifier is not restricted by the parser, but `C` denotes the platform C ABI. An extern parameter list may end with `...` to denote a variadic function. The modifier `header` may appear as an additional convention modifier, as in `extern (C, header) fn ...`; it instructs the compiler to include the corresponding C header. Every extern function is trust-requiring, regardless of the declared signature.

An external value is declared with `extern val`, a name, a type, and an optional symbol string, as in `extern val optind: mut i32 = "optind"`. The optional `mut` modifier makes the C global assignable from Spectre. When the symbol string is omitted, the C symbol is the declared name. The bulk form `extern val (a: T = "a", b: mut T = "b")` declares several external values at once. Reading or writing an external value requires `trust`, either as a `trust` expression or inside a `trust` block.

An external type is declared as `extern type Name = { ... }`. Its fields use the same syntax as an ordinary type, and its layout follows the C ABI. The form `extern "header"` records a header dependency without declaring a function.

The directives `link "name"` and `lib "name"` request linkage against an external library. They are top-level items and may appear inside a platform-conditional block. The exact library search and linker flags are implementation-defined and are handled by the toolchain.

### 4.8 Tests

A test declaration has the form `test { ... }` or `test "name" { ... }`. Tests are top-level items. When the compiler is invoked with `-test`, it suppresses the program entry point, emits a generated entry point that calls the tests of the root module, and prints a summary. A test body is a statement block and is checked in its own scope.

### 4.9 Entry point

The program entry point is a public function named `main` with no parameters. Its declared return type shall be `void` or a signed integer type. The reference compiler supplies the platform argument count and argument vector to the generated entry function even though the source-level `main` declares no parameters; the `@args()` intrinsic exposes the argument vector as a list of references to byte strings. A `main` that is not public or that declares parameters is rejected.

### 4.10 Conditional compilation

A `when` item selects declarations at compile time. At top level, the form is `when selector { items }`, optionally followed by one or more `or when selector { items }` branches and an `otherwise { items }` branch. Only the first matching branch contributes declarations to the program. The selector may name a platform, a platform group, an architecture, or a platform-architecture pair written with a dot. The recognized platform names are `windows`, `linux`, `darwin`, `freebsd`, `openbsd`, `netbsd`, `dragonflybsd`, `android`, `haiku`, `redox`, and `plan9`. The group selectors are `posix` and `unix`, which denote the POSIX platforms, and `bsd`, which denotes the BSD platforms. Architecture selectors include `x86_64`, `aarch64`, and `riscv64`; `amd64` is normalized to `x86_64` and `arm64` to `aarch64`.

The statement form of `when` additionally supports generic conditions. The forms `when T in (A, B)`, `when T !in (A, B)`, `when T is A`, chained conditions of the form `when T is A && U is B`, and the wildcard `when _` are selected during generic monomorphization. The statement-level `when` also supports the platform form with `or when` and `otherwise`. All branches of a generic `when` are checked, and the selected branch is emitted during specialization.

### 4.11 Scoping and name resolution

A scope is introduced by a function body, an `if` branch, a `for` body, a `defer` body, a `guard` else body, a match arm, a `trust` block, and a test body. Name lookup proceeds from the innermost scope outward and from the newest declaration backward within a scope. A declaration may shadow a declaration in an outer scope. A duplicate declaration in the same scope is an error. The loop variable of a C-style `for` may not shadow an existing variable. A `for ... in` binding is immutable.

Unused local variables are errors. A mutable local that is never mutated is an error. The identifier `_` is exempt from both diagnostics and denotes a discarded value. Unused tagged-union bindings are errors unless named `_`.

## 5. Expressions

### 5.1 Primary expressions

A primary expression is an integer literal, a floating-point literal, a string literal, a character literal, `true`, `false`, `none`, a parenthesized expression, an anonymous struct literal, an argument pack, a bracket expression list, an intrinsic, or an identifier optionally followed by `{}` for zero initialization.

A brace-enclosed expression list is interpreted as an anonymous struct literal when its first element has the form `identifier: expression`; otherwise it is an argument pack. The expression `T{}` is zero initialization, not an empty struct literal. A named struct literal has the form `Type{field: value, ...}`; the type name may be qualified, as in `module.Type{...}`. Commas between struct-literal fields are optional, and a trailing comma is permitted. A bracket expression list has the form `[e1, e2, ...]`; commas are required between elements and a trailing comma is permitted. It denotes a fixed array literal and cannot denote a list.

An intrinsic expression begins with `@`, followed by an identifier, an optional type-argument list, and an optional argument list. The intrinsics `@addr`, `@deref`, `@sizeof`, `@bitcast`, and `@zeroed` have dedicated forms. `@addr(e)` produces a reference to the place denoted by `e`; the place shall be mutable. `@deref(p)` produces the value referred to by `p`. `@sizeof(T)` takes a type, not an expression. `@bitcast(e, T)` takes a value followed by a type. `@zeroed` may be written without an argument list or with an empty one. All other intrinsics take parenthesized expression arguments and may carry a type-argument list. Intrinsic names and arities are validated by semantic analysis.

### 5.2 Postfix expressions

A postfix expression applies a sequence of postfix operations to a primary expression. The operations are field access `.name`, function call `(arguments)`, named struct-literal construction, error propagation `?`, cast `as Type`, and generic instantiation `[T1, T2, ...]`. Postfix operations associate from left to right. A field name shall be an identifier and therefore cannot be a reserved word. A call may have an empty argument list or a trailing comma.

There is no subscript or indexing operator. A bracketed type list after an expression is always a generic instantiation. Consequently, `f[i64]` is an instantiation of the generic function `f`, while `f[1]` is not a valid type argument and is rejected. A cast binds more tightly than a prefix operator and more tightly than arithmetic: `-x as i64` denotes `-(x as i64)`, and `x + y as i64` denotes `x + (y as i64)`. Casts associate left to right, so `x as T as U` denotes `(x as T) as U`.

### 5.3 Prefix expressions

The prefix operators are logical negation `!`, its alias `not`, arithmetic negation `-`, bitwise complement `~`, `trust`, `some`, `ok`, and `err`. Prefix operators associate from right to left. There is no unary `+`. The `trust` form applies to an expression, not to a statement block, and has no runtime effect; it permits operations that the trust analysis would otherwise reject. The forms `some e`, `ok e`, and `err e` construct option and result values. The form `ok()` constructs a success value with no payload.

### 5.4 Binary operators and precedence

The binary operators, from lowest to highest precedence, are logical disjunction `||`, logical conjunction `&&`, comparison `==`, `!=`, `<`, `>`, `<=`, `>=`, bitwise or `|`, bitwise exclusive or `^`, bitwise and `&`, shifts `<<` and `>>`, addition and subtraction `+` and `-`, and multiplication, division, and remainder `*`, `/`, and `%`. All binary levels are left-associative. Prefix operators bind less tightly than postfix operations and more tightly than every binary operator.

| Precedence | Operators                                          | Associativity |
| ---------- | -------------------------------------------------- | ------------- |
| lowest     | `\|\|`                                             | left          |
|            | `&&`                                               | left          |
|            | `==` `!=` `<` `>` `<=` `>=`                        | left          |
|            | `\|`                                               | left          |
|            | `^`                                                | left          |
|            | `&`                                                | left          |
|            | `<<` `>>`                                          | left          |
|            | `+` `-`                                            | left          |
|            | `*` `/` `%`                                        | left          |
|            | prefix `!` `not` `-` `~` `trust` `some` `ok` `err` | right         |
| highest    | postfix `.` call `{}` `?` `as` `[...]`             | left          |

The precedence of bitwise operators is higher than that of comparisons. Therefore `x & 1 == 0` parses as `(x & 1) == 0`. Shifts bind more tightly than addition, so `x << 1 + 1` parses as `(x << 1) + 1`. Parentheses may be used to override any precedence.

### 5.5 Arithmetic, comparison, and logical semantics

Integer addition, subtraction, multiplication, division, and remainder follow machine arithmetic for the operand representation. There is no overflow trap and no checked division. Floating-point arithmetic follows the IEEE 754 semantics of the target representation, with `f32` operands promoted to the working double precision where required by the operation and narrowed when stored.

Comparison operators yield a truth value. Scalar values are compared by value. Reference values are compared by address. The core `==` and `!=` operators do not perform structural comparison of strings, lists, or arbitrary aggregates; library functions provide the corresponding structural operations. Option and result values are compared by their tag and payload when the code generator recognizes the comparison.

Logical conjunction and disjunction short-circuit: the right operand is evaluated only when the left operand does not determine the result. A condition is true when its value is nonzero. The operators `&`, `|`, and `^` are bitwise and do not short-circuit. The operator `>>` is a machine right shift; the language does not distinguish arithmetic from logical right shift at the source level.

### 5.6 Casts and representation conversions

A cast expression `e as T` requests a conversion to type `T`. Numeric conversions between integer and floating-point types are permitted. Integer-width and signedness conversions are performed without a representability check. A cast between reference types is permitted; a cast from a reference to an integer representation is permitted; a cast from an integer to a reference is permitted only through an explicit cast. Casting a value to a non-numeric type when the source is definitely numeric is rejected statically, as is casting a definitely pointer-valued expression to a numeric type in the cases recognized by the checker.

The intrinsic `@bitcast(value, T)` is the representation-level conversion form. It is classified as trust-requiring and does not perform a numeric conversion. Its result type is `T`.

### 5.7 Calls and argument packs

A call expression has the form `callee(arg1, arg2, ...)`. The callee may be a function name, a namespaced method, a field selection that denotes a function value, or a generic instantiation. Arguments are comma-separated; a trailing comma is permitted. Argument types are checked against the declared parameter types where the checker can determine them. A mutable argument shall denote a mutable place, and an immutable value cannot be passed to a `mut` parameter.

A parameter declared with type `argspack` accepts a brace-enclosed pack, as in `format("{s} {s}", {"a", "b"})`. The pack may contain heterogeneous values. Argument packs are also accepted by the variadic formatting intrinsics. The `argspack` type is meaningful only in parameter position in a function declaration.

### 5.8 The trust expression

The expression `trust e` evaluates `e` in a trusted context and returns its value. The expression has no runtime representation and does not itself perform a check. Its purpose is to discharge the static obligation created by a trust-requiring operation. A `trust` expression that contains no trust-requiring operation, or that occurs in a context that is already trusted, is diagnosed as redundant.

## 6. Statements and Control Flow

### 6.1 Blocks and sequencing

A statement block is a brace-enclosed sequence of statements. Statements are executed in source order. A block introduces a scope. There is no statement terminator and no automatic semicolon insertion. The statement forms are value declarations, assignments and compound assignments, `return`, `pre`, `post`, `guarded pre`, `guarded post`, `guard`, `if`, `for`, `defer`, `break`, `continue`, `assert`, `when`, `match`, `trust` blocks, and expression statements.

### 6.2 Bindings and assignment

A local value declaration has the form `val name = expression` or `val name: T = expression`. The binding is immutable unless the type is written with the `mut` modifier, as in `val name: mut T = expression`. A local declaration always introduces a binding; the reference implementation records the declared type when it is explicit and otherwise infers a limited type from the initializer.

An assignment statement has the form `place = expression`. The place shall be a mutable binding, a mutable field of a mutable place, or a dereference through a mutable reference. Compound assignment forms are `place += expression`, `place -= expression`, `place++`, and `place--`. The increment and decrement forms are statements, not expressions, and are also accepted in the post clause of a C-style `for`. The reference implementation does not permit chained assignment as a single expression.

### 6.3 Conditional statements

The conditional statement is `if condition { ... }`, followed by zero or more `elif condition { ... }` branches and an optional `else { ... }` branch. Parentheses around the condition are optional. Every branch body shall be a brace-enclosed block. The spelling `else if` is not part of the language; the only spelling is `elif`.

The form `if e is some x { ... }`, `if e is none { ... }`, `if e is ok x { ... }`, and `if e is err x { ... }` is a conditional pattern-binding form. The binding introduced by `some`, `ok`, or `err` is available in the corresponding branch. The `is` form is recognized after the initial `if`; it is not recognized after `elif`. The `none` form takes no binding.

### 6.4 Iteration

The form `for { ... }` is an unconditional loop. The form `for condition { ... }` loops while the condition is true. The form `for name in expression { ... }` iterates over the elements of a list-like value; the binding is immutable. The form `for name = init, condition, post { ... }` is a C-style loop. The post clause shall use one of `++`, `--`, `+= expression`, or `-= expression`. Commas separate the three clauses; semicolons are not accepted. Optional parentheses may enclose the loop header. The language has no `while`, `do`, or `loop` construct.

### 6.5 Pattern matching

The statement `match expression { ... }` selects an arm according to the form of the subject. The arm forms are `some x => { ... }` and `none => { ... }` for an option, `ok x => { ... }` and `err e => { ... }` for a result, `Enum.Variant => { ... }` for an enumeration, `"string" => { ... }` for a string, a constant expression followed by `=>` for a scalar value, and a tagged-union variant with zero or more payload bindings followed by `=>` for a tagged union. Every arm requires `=>` and a brace-enclosed body. Arms are not comma-separated. An `else => { ... }` arm may appear last in the enumeration, string, value, tagged-union, and generic-union forms.

A match is a statement, not an expression. If no arm matches and no `else` arm is present, execution continues after the match. Exhaustiveness is not required for well-formedness. For the purpose of definite-return analysis, a match is treated as exhaustive only when the checker can establish that every variant or value is covered; the details are implementation-defined. Result and option matches accept at most the two corresponding arms.

### 6.6 Guard statements

The statement `guard condition else { ... }` evaluates the condition. If the condition is true, execution continues after the guard. If it is false, the else block is executed. The else block shall end with a `return` statement. A guard is therefore a concise form of an early-exit check and is commonly used at the beginning of a function to establish a precondition.

### 6.7 Deferred statements

The statement `defer { ... }` registers a block to be executed when the enclosing function returns. Deferred blocks execute in reverse order of registration. The reference implementation emits deferred blocks at every explicit `return` and at the implicit return at the end of a function. It does not emit them when control leaves through `break`, `continue`, or the immediate return generated by the `?` operator. A deferred block is function-scoped; it is not executed at the end of the block in which it appears.

### 6.8 Return, break, and continue

The statement `return` returns from the enclosing function. The form `return expression` returns the value of the expression. The form `return` without an expression is permitted; in a non-void function it is treated as returning a zero value by the reference implementation. A non-void function shall have a return on every control-flow path that reaches the end of the function.

The statements `break` and `continue` transfer control to the innermost enclosing loop. They are not expressions. The reference implementation does not validate that they occur inside a loop before code generation.

### 6.9 Assertions

The statement `assert expression` evaluates the expression and aborts the program if the result is false. An assertion is checked in both ordinary and release builds. Failure writes a diagnostic containing the source line to standard error and terminates the process with exit status one.

### 6.10 When statements

The statement `when selector { ... }` selects a branch at compile time. The platform form supports `or when selector { ... }` branches and an `otherwise { ... }` branch, exactly as the item form does. The generic form supports `when T in (A, B)`, `when T !in (A, B)`, `when T is A`, and conjunctions such as `when T is A && U is B`. The wildcard form `when _ { ... }` is selected when no other generic condition matches. All branches of a generic `when` are checked, and only the selected branch is emitted.

### 6.11 Trust blocks

The statement `trust { ... }` executes its block in a trusted context. It has no runtime effect. It is used when a sequence of statements contains trust-requiring operations that the surrounding function does not otherwise admit. A `trust` block that contains no trust-requiring operation, or that occurs in an already trusted context, is diagnosed as redundant.

### 6.12 Contract statements

The statements `pre { ... }`, `post { ... }`, `guarded pre { ... }`, and `guarded post { ... }` introduce contract clauses. Each clause in the block consists of an optional label followed by a colon and an expression. Clauses are separated by whitespace; commas and semicolons are not accepted inside a contract block. A contract statement may appear at any point in a function body, and its check is executed when control reaches that statement. By convention, preconditions are placed at the beginning of a function and postconditions immediately before return.

## 7. Design by Contract and Trust

### 7.1 Contract clauses

A contract clause is a boolean condition with an optional label. The label is an identifier followed by a colon. The condition is an expression evaluated as a truth value: zero is false and nonzero is true. The label is used in the runtime diagnostic when the condition fails. A contract block contains one or more clauses. The forms are `pre` for a precondition, `post` for a postcondition, `guarded pre` for a precondition that persists in release builds, and `guarded post` for a postcondition that persists in release builds.

### 7.2 Preconditions

A precondition states an obligation on the caller and on the program state at the point at which the contract statement executes. When control reaches a precondition, the condition is evaluated. If it is false, the program writes a diagnostic of the form `spectre: precondition '<label>' violated in function '<name>'` to standard error, flushes output, and terminates with exit status one. If the condition is true, execution continues with the next statement.

The reference implementation attempts a compile-time check at direct call sites. It collects unconditional precondition statements that appear directly in the callee body, or in the selected branch of a top-level `when` in the callee body. If the call arguments and the collected conditions are constants in the supported constant-evaluation fragment and a condition evaluates to false, the call is a compile-time error. Conditions outside that fragment are deferred to runtime. This optimization does not change the dynamic meaning of the precondition.

### 7.3 Postconditions

A postcondition states an obligation on the function at the point at which the contract statement executes. Its runtime treatment is identical to that of a precondition, except that the diagnostic identifies a postcondition. The reference implementation does not perform compile-time evaluation of postconditions. A postcondition placed after a `return` statement is unreachable and therefore has no effect, so postconditions are conventionally placed immediately before the return.

### 7.4 Guarded contracts and release mode

An ordinary pre- or postcondition is checked when the program is compiled in ordinary mode and is omitted when the program is compiled with `-release`. A guarded pre- or postcondition is checked in both modes. The choice is made at code generation time, not by a runtime flag. Contract failure always terminates the process; there is no recoverable exception or result value for a failed contract.

### 7.5 Type invariants

An invariant is declared inside a type body with `invariant { ... }`. It contains one or more boolean expressions separated by whitespace. The invariant expresses a property of every value of the type. The reference implementation applies the following checks.

A constructor expression whose destination type is explicit and whose field values are statically known is checked at compile time. If an invariant expression evaluates to false in the supported constant-evaluation fragment, the declaration is rejected with an invariant violation. If the fields or the invariant are not statically known, no compile-time error is produced.

At runtime, the reference implementation checks registered invariants at the return points of a function for parameters declared directly as `mut T`, where `T` is a named type with an invariant. It does not insert an invariant check at function entry, on ordinary value parameters, on local bindings, on field assignment, on increment or decrement, or on list mutation. In release mode, runtime invariant checks are omitted. An invariant violation writes `spectre: invariant violated for type '<T>' in '<function>'` to standard error and terminates the process with exit status one.

These rules describe the current reference implementation precisely; an implementation that enforces invariants more eagerly remains conforming provided that it does not accept a program that violates an invariant obligation.

### 7.6 Compile-time evaluation

The static contract checker evaluates a restricted fragment of expressions. The fragment includes integer, boolean, and character literals; cast and trust wrappers; identifiers whose values are known from the call arguments or from a constructor's field values; unary minus and logical negation; the comparisons `>`, `>=`, `<`, `<=`, `==`, and `!=`; logical conjunction and disjunction; and addition, subtraction, and multiplication. It does not include division, remainder, bitwise operations, shifts, floating-point arithmetic, string operations, or arbitrary function calls. A condition outside the fragment is not rejected; its check is deferred to runtime when the construct is checked dynamically.

### 7.7 Assertions

An assertion is a runtime obligation that is independent of the contract annotations. The statement `assert e` checks `e` in every build mode. It is therefore the appropriate construct for internal consistency checks that must survive release compilation. An assertion failure identifies the source line and terminates the process with exit status one.

### 7.8 Trust and the trust boundary

The trust discipline separates operations whose correctness depends on an external argument from operations whose correctness the language can check. The boundary is crossed by a trust-requiring operation, which may be an unsafe intrinsic, an extern function, or a function whose body is marked `trust`. Such an operation may be used only inside a trusted context. A trusted context is established by a function whose body begins with `trust`, by a `trust` expression, or by a `trust` block. Trust contexts nest and compose.

A function declared with `= trust { ... }` is a trusted function. A call to a trusted function from an untrusted context is rejected unless the call is itself wrapped in `trust`. An extern function is always trusted, regardless of its declared signature. A trust-requiring intrinsic is rejected outside a trusted context. The `trust` construct is a static permission, not a runtime capability: it neither suppresses contract checks nor changes the generated code for the trusted expression or block.

A `trust` expression or block that performs no trust-requiring operation is diagnosed as redundant. A `trust` expression or block inside an already trusted context is likewise diagnosed as redundant. These diagnostics enforce the principle that trust should be localized to the boundary at which an external guarantee is introduced.

### 7.9 Trust-requiring operations

The reference checker classifies the following intrinsics as trust-requiring: `@addr`, `@bitcast`, `@deref`, `@dprintf`, `@fmtalloc`, `@free`, `@load`, `@load8`, `@memcpy`, `@memmove`, `@memset`, `@print`, `@ptradd`, `@realloc`, `@store`, and `@store8`. The intrinsic `@alloc` is not in this list; allocation itself is permitted outside a trust context in the reference implementation. The wider-width load and store intrinsics `@load16`, `@load32`, `@load64`, `@store16`, `@store32`, and `@store64` are likewise not in the trust-requiring list. The classification is a property of the reference checker; an implementation may classify additional operations as trust-requiring, but shall not permit a program to call an extern function or execute an operation that is specified as trust-requiring outside a trusted context.

### 7.10 A contract-bearing type

The following declaration illustrates a type invariant, a guarded precondition, and a postcondition on a mutating method. The fields are mutable because the method assigns them, and the invariant states the property that every `Window` value is expected to maintain.

```spectre
pub type Window = {
    start: mut usize
    len: mut usize
    invariant {
        len > 0
        start + len > start
    }
}

pub fn (Window) resize(w: mut self, len: usize) void = {
    guarded pre {
        positive: len > 0 as usize
    }
    w.len = len
    post {
        positive: w.len > 0 as usize
    }
}
```

The `guarded` modifier causes the precondition to remain in a release build. The invariant is checked at the return of `resize` because the receiver is a direct `mut Window` parameter. The postcondition is checked at the point at which it appears, immediately before the implicit return.

## 8. Memory Model and Allocation

### 8.1 Object model

Spectre values are stored in three broad classes of storage: global storage associated with top-level values, automatic storage associated with function activations, and dynamically allocated storage addressed by references. The language does not define a garbage collector, an ownership type, a borrow checker, or a lifetime parameter. The lifetime of a dynamically allocated object is determined by the program's calls to allocation and deallocation operations.

A reference is a raw address. The language does not guarantee that a reference is non-null, that it is aligned for its referent type, that it is dereferenceable, or that its referent is still alive. These properties are established by the program through construction, contracts, guards, and the trust boundary. This is the central low-level obligation of the language.

### 8.2 Alignment and representation

A value of a primitive type is stored at an address aligned for that type in the reference implementation. Non-extern named types are packed, so a field may be unaligned for its nominal type; the compiler's load and store operations use the representation required by the field access. Extern types use the C ABI layout and alignment. A list element occupies one 64-bit slot regardless of the element type; values narrower than 64 bits are widened when stored and narrowed when loaded according to their static type. A caller that performs raw pointer arithmetic is responsible for respecting the layout and alignment of the objects it addresses.

### 8.3 Memory intrinsics

The memory intrinsics operate on raw addresses and byte counts. They do not validate their arguments. The following table summarizes their forms and results. The arity column lists the number of value arguments; an optional type-argument list is noted separately.

| Intrinsic           |  Arity | Result       | Semantics                                         |
| ------------------- | -----: | ------------ | ------------------------------------------------- |
| `@alloc(n)`         |      1 | `ref void`   | allocate `n` bytes from the C heap; no zeroing    |
| `@free(p)`          |      1 | `void`       | release a heap allocation                         |
| `@realloc(p, n)`    |      2 | `ref void`   | resize a heap allocation to `n` bytes             |
| `@sizeof(T)`        | 1 type | `usize`      | size of a type in bytes, computed at compile time |
| `@addr(e)`          |      1 | `ref T`      | address of a mutable place                        |
| `@deref(p)`         |      1 | `T`          | value referred to by a reference                  |
| `@load(p)`          |      1 | machine word | load a 64-bit word                                |
| `@load8(p)`         |      1 | `u8`         | load and zero-extend a byte                       |
| `@load16(p)`        |      1 | `u16`        | load and zero-extend a halfword                   |
| `@load32(p)`        |      1 | `i32`        | load and sign-extend a 32-bit word                |
| `@load64(p)`        |      1 | machine word | load a 64-bit word                                |
| `@store(p, v)`      |      2 | `void`       | store a scalar; f64/f32 use their natural width   |
| `@store8(p, v)`     |      2 | `void`       | store a byte                                      |
| `@store16(p, v)`    |      2 | `void`       | store a halfword                                  |
| `@store32(p, v)`    |      2 | `void`       | store a 32-bit word                               |
| `@store64(p, v)`    |      2 | `void`       | store exactly one 64-bit word                     |
| `@ptradd(p, n)`     |      2 | `ref T`      | add `n` bytes to a reference                      |
| `@memcpy(d, s, n)`  |      3 | `void`       | copy `n` bytes; regions shall not overlap         |
| `@memmove(d, s, n)` |      3 | `void`       | copy `n` bytes; regions may overlap               |
| `@memset(d, v, n)`  |      3 | `void`       | fill `n` bytes with the low byte of `v`           |
| `@bitcast(v, T)`    |      2 | `T`          | representation-level conversion                   |
| `@zeroed` or `T{}`  |      0 | `T`          | zero-initialize a value of the destination type   |

The intrinsic `@alloc` returns a null reference when allocation fails; the caller shall check the result if failure is possible. The intrinsic `@realloc` likewise returns a null reference on failure. The intrinsics `@free`, `@memcpy`, `@memmove`, and `@memset` return no useful value. The intrinsic `@ptradd` performs byte arithmetic, not element arithmetic; a caller that wants element addressing multiplies the index by the element size.

### 8.4 List intrinsics

The list intrinsics operate on the list representation described in Section 3.8. The checked operation is `@get`; the other indexed operations require the caller to establish the index bound. The allocator-backed constructors require a type that provides the allocation, deallocation, and reallocation operations described in Section 8.6.

| Intrinsic              |    Arity | Result               | Semantics                                                           |
| ---------------------- | -------: | -------------------- | ------------------------------------------------------------------- |
| `@len(l)`              |        1 | `usize`              | current number of elements                                          |
| `@capacity(l)`         |        1 | `usize`              | current capacity in elements                                        |
| `@get(l, i)`           |        2 | `option[T]`          | element at `i`, or `none` when out of bounds                        |
| `@set(l, i, v)`        |        3 | `void`               | replace the element at `i`; no bounds check                         |
| `@append(l, v)`        |        2 | `void`               | append `v`, growing the buffer as needed                            |
| `@insert(l, i, v)`     |        3 | `void`               | insert `v` at `i`, shifting later elements                          |
| `@remove(l, i)`        |        2 | `T`                  | remove and return the element at `i`                                |
| `@reserve(l, n)`       |        2 | `void`               | ensure capacity for at least `n` elements                           |
| `@listfree(l)`         |        1 | `void`               | release the list storage                                            |
| `@list(ac, e...)`      | variadic | `result[list[T], E]` | allocator-backed list from elements                                 |
| `@list(ac)`            |        1 | `result[list[T], E]` | empty allocator-backed list                                         |
| `@listalloc[A, T](ac)` |        1 | `result[list[T], E]` | empty list using allocator type `A`; `T` is optional when inferable |

The core constructors of dynamic lists are `@list` and `@listalloc`. The bracket expression list `[e1, e2, ...]` is a fixed array literal and does not allocate a list. An allocator-backed list records its allocator descriptor and releases its buffer through that allocator when `@listfree` is called. The `@append`, `@insert`, `@remove`, `@reserve`, and `@set` operations do not return an error value; a caller that requires allocation-failure handling shall use an allocator-backed construction form that returns a result, and shall establish the capacity before performing unchecked mutations.

### 8.5 Formatting, I/O, and process intrinsics

The following intrinsics provide formatting, standard I/O, argument access, and compile-time diagnostics. They are part of the core intrinsic set even when their implementation delegates to the platform C library.

| Intrinsic                      |    Arity | Result                    | Semantics                                        |
| ------------------------------ | -------: | ------------------------- | ------------------------------------------------ |
| `@print(fmt, pack)`            |   1 or 2 | `void`                    | formatted output to standard output              |
| `@dprintf(fd, fmt, pack)`      |   2 or 3 | `void`                    | formatted output to a file descriptor            |
| `@snprintf(buf, n, fmt, pack)` |   3 or 4 | integer                   | formatted output into a buffer of `n` bytes      |
| `@puts(s)`                     |        1 | `void`                    | write a NUL-terminated string and a newline      |
| `@flush()`                     |        0 | `void`                    | flush buffered output                            |
| `@stdin`                       |        0 | stream                    | standard input stream value                      |
| `@stdout`                      |        0 | stream                    | standard output stream value                     |
| `@stderr`                      |        0 | stream                    | standard error stream value                      |
| `@fgets(buf, n, stream)`       |        3 | reference                 | read at most `n - 1` bytes into `buf`            |
| `@getchar()`                   |        0 | integer                   | read one character from standard input           |
| `@args()`                      |        0 | `list[ref u8]`            | process argument vector                          |
| `@argsalloc[A](ac)`            |        1 | `result[list[ref u8], E]` | argument vector allocated with `A`               |
| `@fmtalloc(fmt, pack)`         |   1 or 2 | `ref u8`                  | heap-allocated formatted string                  |
| `@fmtalloc[A](ac, fmt, pack)`  |   2 or 3 | `result[ref u8, E]`       | allocator-backed formatted string                |
| `@strlen(s)`                   |        1 | `i64`                     | length of a NUL-terminated string                |
| `@strcmp(a, b)`                |        2 | integer                   | lexical comparison of two NUL-terminated strings |
| `@atoi(s)`                     |        1 | integer                   | parse a decimal integer                          |
| `@call(f, ...)`                | variadic | depends on `f`            | indirect call through a function value           |
| `@staticerror(message)`        |        1 | no value                  | abort compilation with a message                 |
| `@embed(path)`                 |        1 | `ref u8`                  | reference to compile-time file contents          |
| `@embedlen(path)`              |        1 | `usize`                   | length of compile-time file contents             |

The formatting intrinsics accept a format string and an optional brace-enclosed argument pack. The format language is the platform `printf`-style language extended by Spectre's brace directives; the standard-library formatting functions are the recommended interface for code that must remain portable across the supported targets. The intrinsic `@staticerror` is evaluated while the compiler generates the containing function and therefore aborts compilation if that function is generated, regardless of whether the runtime path would reach it. The paths accepted by `@embed` and `@embedlen` are resolved relative to the source file that contains the intrinsic. `@embed` yields a reference to the exact file bytes and `@embedlen` yields their byte count; the embedded data is not implicitly followed by a NUL byte.

### 8.6 The allocator protocol

The language does not define an allocator type in its grammar. It defines the operations through which allocator-threaded APIs are expressed, and the standard library provides the canonical protocol. The protocol is a design convention rather than a special type class in the compiler; its obligations are expressed through specifications, generic constraints, and ordinary function parameters.

The standard-library specification `Allocator` declares the operations required for bump allocation: a constructor `new(capacity) result[Allocator, AllocError]`, a deallocator `free(a) void`, and an allocator `alloc(a, size) option[ref void]`. The specification `GeneralAllocator` declares `alloc`, `free`, and `realloc` operations with signatures suitable for growable storage. The standard-library types `Arena`, `Stack`, and `ScopedArena` implement bump or scoped allocation; `ListArena` adapts a `ScopedArena` to the `GeneralAllocator` protocol; `HeapAllocator` implements general heap allocation. The name `ScopedArenaAllocator` denotes an empty specification used as an opaque handle for a `ScopedArena`.

An allocator is threaded through an API as an ordinary value parameter. The idiomatic form places the allocator first and makes the API generic over the allocator type, as in `fn f[A: GeneralAllocator](ac: A, ...)`. A caller supplies a concrete allocator as an explicit type argument, as in `f[HeapAllocator](heap, ...)`, or requests inference with `f[_](heap, ...)`. Passing an allocator by value copies its handle; the handle refers to shared backing state. Deinitialization is performed explicitly, either by a `free` method on the allocator or by releasing the larger scope that owns the backing storage.

The allocator-aware list and formatting intrinsics require an allocator type that provides allocation, deallocation, and reallocation. The bump-only allocators `Arena` and `Stack` do not provide individual deallocation or reallocation and therefore are not suitable as the type argument to `@listalloc` or to the allocator-backed `@fmtalloc`. They are suitable for `Allocator`-constrained APIs that only allocate and for `ScopedArena`-style phase management. `ListArena` provides allocation, no-op deallocation, and copying reallocation over a scoped arena, so it is suitable as the type argument to `@listalloc` for lists whose storage is released with the underlying scoped arena.

### 8.7 Deallocation and ownership

The language has no automatic destructor and no ownership type. The owner of an allocation is the part of the program that is responsible for calling the matching deallocation operation. The standard library documents the ownership of the values it returns; for example, a list returned by an allocator-aware constructor is released with `@listfree`, while an arena-backed string is released when the arena is reset or freed.

The `defer` statement is the principal structured cleanup mechanism. A function that acquires a resource registers its release in a `defer` block immediately after acquisition, so that every later return path releases the resource. Because the reference implementation does not execute deferred blocks on `break`, `continue`, or `?`, a function that must release a resource on those paths shall use an explicit cleanup path or avoid those transfers while the resource is live.

A raw allocation from `@alloc` is released with `@free`; a raw allocation resized with `@realloc` remains owned by the same binding. The allocator-aware constructors release through the allocator descriptor recorded in the object. Mixing an object with an allocator other than the one that created it is outside the defined behavior of the library protocol.

### 8.8 Allocator-generic API example

The following function threads a `GeneralAllocator` through an API, establishes a precondition on the source string, and states a postcondition on the copied buffer. It allocates through the allocator, copies the source bytes, and returns an option. The caller receives the allocation through the same allocator and releases it with the matching deallocation operation.

```spectre
pub fn duplicate[A: allocators.GeneralAllocator](
    ac: A,
    src: ref u8
) option[ref u8] = trust {
    pre {
        src_present: src != none
    }
    val n: usize = @strlen(src) as usize + 1
    val dst = allocators.alloc_in[A](ac, n)
    match dst {
        some raw => {
            val out: ref u8 = raw as ref u8
            @memcpy(out, src, n)
            post {
                terminated: @load8(@ptradd(out, n - 1)) == 0
            }
            return some out
        }
        none => {
            return none
        }
    }
}
```

The call site supplies the allocator type explicitly and releases the result through the same allocator.

```spectre
val heap: allocators.HeapAllocator = @zeroed
match duplicate[allocators.HeapAllocator](heap, "spectre") {
    some text => {
        stdio.print("copy: {s}\n", {text})
        allocators.free_in[allocators.HeapAllocator](heap, text as ref void)
    }
    none => {
        stdio.puts("allocation failed\n")
    }
}
```

### 8.9 Arena-scoped allocation example

An arena is suitable for a phase of computation whose entire working set can be released at once. The following function threads an `Allocator` through a file-reading API. The caller owns the arena and releases it with a deferred `free`; the function does not free the returned bytes individually.

```spectre
pub fn file_size[A: allocators.Allocator](ac: A, path: ref u8) option[usize] = trust {
    pre {
        path_present: path != none
    }
    val data = fs.read_all[A](ac, path)
    match data {
        some raw => {
            return some (@strlen(raw as ref u8) as usize)
        }
        none => {
            return none
        }
    }
}
```

A complete program that combines the arena lifetime, the allocator-threaded API, a contract-bearing type, and an option result is the following.

```spectre
val allocators = use("std/allocators")
val fs = use("std/fs")
val stdio = use("std/stdio")

pub type Window = {
    start: mut usize
    len: mut usize
    invariant {
        len > 0
        start + len > start
    }
}

pub fn (Window) resize(w: mut self, len: usize) void = {
    guarded pre {
        positive: len > 0 as usize
    }
    w.len = len
    post {
        positive: w.len > 0 as usize
    }
}

pub fn file_size[A: allocators.Allocator](ac: A, path: ref u8) option[usize] = trust {
    pre {
        path_present: path != none
    }
    val data = fs.read_all[A](ac, path)
    match data {
        some raw => {
            return some (@strlen(raw as ref u8) as usize)
        }
        none => {
            return none
        }
    }
}

pub fn main() i32 = trust {
    val arena: mut allocators.Arena = allocators.Arena.new(65536 as usize)?
    defer {
        allocators.Arena.free(arena)
    }
    val w: mut Window = Window{start: 0 as usize, len: 8 as usize}
    Window.resize(w, 16 as usize)
    match file_size[allocators.Arena](arena, "sx.mod") {
        some n => {
            stdio.print("window={d} sx.mod={d}\n", {w.len, n})
            return 0
        }
        none => {
            stdio.puts("could not read sx.mod\n")
            return 1
        }
    }
}
```

The program illustrates the intended division of responsibility. The arena owns the backing storage and is released once by the `defer` block. The generic `file_size` function receives the arena as an ordinary value parameter and allocates through its `Allocator` interface. The `Window` type carries an invariant, and `resize` carries a guarded precondition and a postcondition. The result of the file operation is an option, so failure is handled explicitly rather than by an unchecked assumption.

### 8.10 Low-level access

A program that requires direct memory access uses the raw memory intrinsics from Section 8.3. Such access is trust-requiring for loads, stores, pointer arithmetic, and memory copying. The programmer is responsible for the validity, alignment, size, and lifetime of every address, and for the absence of overlapping copies where `@memcpy` is used. The trust boundary does not weaken these obligations; it records that the programmer has accepted them at a specific source location.

## 9. Execution Model and Toolchain

### 9.1 Compilation pipeline

The reference compiler performs the following stages. The lexer converts source bytes into tokens. The parser constructs an abstract syntax tree. Module resolution loads imported modules, detects cycles, and assembles the module tree. Semantic analysis registers names, checks declarations and types where implemented, applies the contract and trust checks, and reports diagnostics. Code generation lowers the checked program to QBE intermediate representation, which is then lowered to the target assembly and linked. The alternative backend translates the intermediate representation to C or another target representation and invokes a platform C compiler.

The compiler can print intermediate forms with `-emit-ssa`, `-emit-alt`, and `-emit-asm`. The `ast` command prints the abstract syntax tree, and the `check` command runs the frontend and semantic analysis without generating code. The `port-c` command translates C source into a single Spectre source file. The `bind-c` command generates public Spectre extern bindings from C headers. The `-emit-shim` option writes the QBE runtime shim alongside `-emit-ssa`.

### 9.2 Target selection

The target platform for `when` selection is the host platform unless the compiler is invoked with `-src-target:<os>`. The target architecture is the host architecture. The platform names and group selectors are those listed in Section 4.10. The target selection affects which conditional declarations are analyzed and emitted; it does not change the syntax of the selected branch. The `when` selection is performed during semantic analysis and code generation, so a branch that is not selected is not type-checked as part of the program.

### 9.3 Command-line interface

The compiler is invoked as `spectre <input.sx> [options]` for a single source file, or as `spectre <command> [arguments]` for a toolchain command. The principal commands are `init`, `fmt`, `critic`, `build`, `run`, `check`, `ast`, `doc`, `add`, `remove`, `upgrade`, `restore`, `graph`, `ref`, `clean`, `doctor`, `port-c`, and `bind-c`. The principal compilation options are `-emit-ssa`, `-emit-alt`, `-emit-asm`, `-src-target=<os>`, `-inc-path=<dir>`, `-show-cmd`, `-test`, `-release`, `-dynamic`, `-dll`, `-a` or `-alt`, `-b` or `-bootstrap`, `-c` or `-compile`, `-d:<NAME>=<value>`, `-o <file>`, `-l` or `-loud`, `-v` or `-version`, and `-h` or `-help`. The option `-I:<headers>` is accepted only by the alternative backend.

The `check` command accepts `spectre check <input.sx> [options]`. It runs the frontend and semantic analysis and reports diagnostics without generating code.

The `port-c` command accepts `spectre port-c <input.c|input.h> [-o <file>]`. The `bind-c` command accepts one or more files or directories and writes one `.sx` per input under `./src-out` by default. It uses the host C preprocessor, emits declarations from the target file and its local quoted includes, skips static functions, and can emit scalar object-like macros. Its options include `-o <dir>`, `-normalize`, `-link <name>`, `-target:<os>.<arch>`, `-I <dir>`, `-D <name[=value]>`, and `-U <name>`.

The `-release` option selects release code generation, which omits ordinary pre- and postcondition checks and runtime invariant checks, but retains guarded contracts and assertions. The `-test` option selects the test runner described in Section 4.8. The `-dynamic` option requests dynamic linking instead of static linking where the platform supports it.

### 9.4 Project manifest

A project is described by a file named `sx.mod` in the project root. The manifest fields are `entry`, which names the source file passed to the compiler; `version`, which names the project version; `spectre_version`, which names the required compiler version; `single_flags`, which supplies default flags for standalone compilation of files in the project; and repeated `dep` entries, which name Git dependencies cloned into `lib/`. A `build` block introduces a named build configuration. A build configuration may specify `action`, `flags`, `output`, `entry`, and an `env` block of compile-time environment variables. It may also contain platform or platform-architecture blocks such as `posix { ... }`, `windows { ... }`, `posix.aarch64 { ... }`, or `windows.x86_64 { ... }`. Matching platform blocks override individual fields of the enclosing configuration and are applied in declaration order.

The `spectre_version` field is required by the reference toolchain and shall match the installed compiler version. The manifest syntax is parsed by the toolchain rather than by the language parser; its identifiers and strings are not part of the core expression grammar.

### 9.5 Diagnostics and termination

Static diagnostics are accumulated during semantic analysis. If any error is present, the compiler reports the diagnostics and does not proceed to code generation. The exact diagnostic text is implementation-defined, but a diagnostic identifies the source file and line. A contract failure, an assertion failure, an invariant failure, and a panic terminate the program with a nonzero status; contract and assertion failures use status one. The `@staticerror` intrinsic terminates compilation rather than execution.

Runtime failures do not unwind the stack and do not run deferred blocks. A program that requires cleanup before termination shall perform it before the failing operation or use an explicit check that returns a result instead of a contract or assertion.

## 10. Grammar

The following grammar is expressed in EBNF. Terminals are quoted. Braces `{ ... }` denote zero or more repetitions, brackets `[ ... ]` denote optional elements, and parentheses group alternatives. A trailing comma is accepted in the lists marked by the grammar. The grammar describes the surface language accepted by the reference parser; the static and dynamic rules of the preceding sections determine whether a derivation denotes a well-formed program.

```ebnf
(* Lexical categories *)
IDENT          = letter { letter | digit | "_" } ;
INT            = decimal-digit { decimal-digit }
               | "0x" { hex-digit } ;
FLOAT          = decimal-digit { decimal-digit } ( "." { decimal-digit } | exponent ) ;
exponent       = ( "e" | "E" ) [ "+" | "-" ] { decimal-digit } ;
STRING         = '"' { string-byte } '"' | raw-string ;
CHAR           = "'" char-byte "'" ;

(* Compilation unit *)
unit           = { item } EOF ;
item           = link-item | lib-item | test-item | when-item
               | [ "pub" ] core-item ;
core-item      = extern-decl | spec-decl | fn-decl | type-decl
               | enum-decl | union-decl | val-decl ;

link-item      = "link" STRING ;
lib-item       = "lib" STRING ;
test-item      = "test" [ STRING ] block ;

(* Values and imports *)
val-decl       = "val" ( val-binding | bulk-val ) ;
val-binding    = IDENT [ ":" [ "mut" ] type ] "=" ( use-expr | expr ) ;
use-expr       = "use" "(" STRING ")" ;
bulk-val       = "(" [ bulk-entry { [ "," ] bulk-entry } [ "," ] ] ")" ;
bulk-entry     = IDENT [ ":" [ "mut" ] type ] "=" expr ;

(* Functions *)
fn-decl        = "fn" [ "(" IDENT ")" ] IDENT [ type-params ]
                 "(" [ param { "," param } [ "," ] ] ")" type
                 [ "=" { "inline" | "trust" } block ] ;
type-params    = "[" [ tparam { "," tparam } [ "," ] ] "]" ;
tparam         = IDENT [ ":" type ] ;
param          = IDENT ":" [ "mut" ] type ;

(* Types *)
type           = "[]" type
               | "[" ( INT | dotted-ident ) "]" type
               | "ref" type
               | "mut" type
               | "fn" "(" [ type { "," type } [ "," ] ] ")" type
               | "option" "[" type "]"
               | "result" "[" type "," type "]"
               | "list" "[" type "]"
               | "void"
               | "argspack"
               | dotted-ident ;
dotted-ident   = IDENT { "." IDENT } ;

(* Named types, specifications, enums, and unions *)
type-decl      = "type" [ "(" dotted-ident ")" ] IDENT "="
                 "{" { field | invariant } "}" ;
field          = IDENT ":" [ "mut" ] type [ "," ] ;
invariant      = "invariant" "{" { expr } "}" ;

spec-decl      = "spec" IDENT "=" "{" { [ "pub" ] fn-decl } "}" ;

enum-decl      = "enum" IDENT "=" "{" [ IDENT { [ "," ] IDENT } [ "," ] ] "}" ;

union-decl     = "union" IDENT "=" "{" variant { "|" variant } [ "|" ] "}" ;
variant        = IDENT "(" [ type { "," type } [ "," ] ] ")" ;

(* External declarations *)
extern-decl    = "extern" STRING
               | "extern" "type" [ "(" dotted-ident ")" ] IDENT "="
                 "{" { field | invariant } "}"
               | "extern" "(" IDENT { "," IDENT } ")" "fn" IDENT
                 "(" [ extern-params ] ")" type "=" STRING ;
extern-params  = "..."
               | extern-param { "," extern-param } [ "," "..." ] ;
extern-param   = IDENT ":" type ;

(* Statement blocks and statements *)
block          = "{" { stmt } "}" ;
stmt           = val-stmt
               | "return" [ expr ]
               | contract-stmt
               | guard-stmt
               | if-stmt
               | for-stmt
               | "defer" block
               | "break"
               | "continue"
               | "assert" expr
               | when-stmt
               | match-stmt
               | "trust" block
               | expr [ assign-tail ] ;
val-stmt       = "val" IDENT [ ":" [ "mut" ] type ] "=" expr ;
assign-tail    = "=" expr | "++" | "--" | "+=" expr | "-=" expr ;

contract-stmt  = ( "pre" | "post" | "guarded" ( "pre" | "post" ) )
                 "{" { [ IDENT ":" ] expr } "}" ;
guard-stmt     = "guard" expr "else" block ;

if-stmt        = "if" expr [ "is" ( "some" IDENT | "none"
                 | "ok" IDENT | "err" IDENT ) ] block
                 { "elif" expr block }
                 [ "else" block ] ;

for-stmt       = "for" block
               | "for" [ "(" ] IDENT "in" expr [ ")" ] block
               | "for" [ "(" ] IDENT "=" expr "," expr "," for-post [ ")" ] block
               | "for" [ "(" ] expr [ ")" ] block ;
for-post       = expr ( "++" | "--" | "+=" expr | "-=" expr ) ;

match-stmt     = "match" expr "{" { match-arm } "}" ;
match-arm      = match-pattern "=>" block ;
match-pattern  = "some" IDENT
               | "none"
               | "ok" IDENT
               | "err" IDENT
               | dotted-ident
               | STRING
               | expr
               | IDENT [ "(" ")" | IDENT { [ "," ] IDENT } ]
               | "else" ;

when-stmt      = "when" when-head block
                 { "or" "when" when-head block }
                 [ "otherwise" block ] ;
when-head      = IDENT [ "." IDENT ]
               | when-head "is" type { "&&" IDENT "is" type }
               | when-head ( "in" | "!" "in" )
                 "(" [ type { "," type } [ "," ] ] ")"
               | "_" ;

(* Expressions *)
expr           = or-expr ;
or-expr        = and-expr { "||" and-expr } ;
and-expr       = cmp-expr { "&&" cmp-expr } ;
cmp-expr       = bitor-expr { ( "==" | "!=" | "<" | ">" | "<=" | ">=" ) bitor-expr } ;
bitor-expr     = bitxor-expr { "|" bitxor-expr } ;
bitxor-expr    = bitand-expr { "^" bitand-expr } ;
bitand-expr    = shift-expr { "&" shift-expr } ;
shift-expr     = add-expr { ( "<<" | ">>" ) add-expr } ;
add-expr       = mul-expr { ( "+" | "-" ) mul-expr } ;
mul-expr       = unary-expr { ( "*" | "/" | "%" ) unary-expr } ;
unary-expr     = ( "!" | "not" | "-" | "~" | "trust" | "some" | "ok" | "err" )
                 unary-expr
               | postfix-expr ;
postfix-expr   = primary { postfix-op } ;
postfix-op     = "." IDENT
               | "(" [ expr { "," expr } [ "," ] ] ")"
               | "{" struct-fields "}"
               | "?"
               | "as" type
               | "[" [ type-arg { "," type-arg } [ "," ] ] "]" ;
type-arg       = "_" | type ;

primary        = INT | FLOAT | STRING | CHAR
               | "true" | "false" | "none"
               | "(" expr ")"
               | "{" ( struct-fields | expr-list ) "}"
               | "[" [ expr-list ] "]"
               | intrinsic
               | IDENT [ "{" "}" ] ;
struct-fields  = IDENT ":" expr { [ "," ] IDENT ":" expr } ;
expr-list      = expr { "," expr } [ "," ] ;

intrinsic      = "@" IDENT [ "[" [ type-arg { "," type-arg } [ "," ] ] "]" ]
                 [ "(" [ expr { "," expr } [ "," ] ] ")" ] ;
```

The grammar admits several forms that the semantic rules restrict further. A named struct literal is recognized only when the first field has the form `IDENT ":"`; an argument pack is the alternative. A bracket expression list in primary position is a fixed array literal and does not construct a list. A generic instantiation is written with brackets after an expression, and the language has no indexing expression. The `is` form of `if` is recognized only after the initial `if`. The `match` form is selected by the shape of its first arm, and every arm body is a block. The `when` head is selected by the shape of the condition; the wildcard `_` form applies only to a generic `when` with that selector.

## 11. Language Restrictions and Implementation Notes

The core language does not provide closures, nested function declarations, generic type declarations, untagged unions, block comments, semicolon statement terminators, array indexing syntax, `while`, `do`, `loop`, operator overloading, exception handling, automatic memory management, ownership types, or borrow checking. A program that requires any of these facilities expresses the corresponding behavior through the constructs specified above or through the standard library.

The reference implementation currently accepts a type-parameter list on a function declaration only; a type declaration, enumeration, union, or extern declaration has no type parameters. It accepts only tagged unions. It recognizes `option`, `result`, `list`, `void`, and `argspack` by identifier text in type position, so those names are not available as ordinary named types in that position. It treats the token `;` as a lexical token but not as a grammatical statement separator. It does not provide an indexing expression, so a bracket form after an expression is always a generic instantiation; a bracket expression list in primary position is a fixed array literal.

The reference implementation's static analysis is intentionally conservative in scope. It checks duplicate definitions, undefined names, unknown types in function signatures, call arity and argument compatibility where the types are known, mutability restrictions on assignment and on passing arguments to `mut` parameters, the public visibility and arity of `main`, the requirement that a `guard` else block ends in `return`, and the requirement that a non-void function returns on every path. It does not perform a general type-compatibility check on every assignment initializer or return expression; code generation converts the supplied value to the destination representation. It does not check the representability of an integer conversion, the validity of a reference, the alignment of a raw address, the bounds of an unchecked list index, or the lifetime of an allocation. These are obligations of the program and are the reason for the contract and trust disciplines.

The reference implementation's runtime contract checks are emitted at the position of the contract statement, not automatically at function entry and exit. A postcondition placed after a `return` is unreachable. The runtime invariant check is emitted for direct `mut T` parameters of named types with invariants at function return points, and is omitted in release builds. Assertions are emitted in every build mode. The `?` operator returns immediately on an error and does not execute deferred blocks. These behaviors are described here so that a program's observable behavior can be predicted from its source text.
