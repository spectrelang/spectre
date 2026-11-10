# Spectre

![License](https://img.shields.io/badge/license-GPLv3-red)
![Status](https://img.shields.io/badge/status-alpha-red)
![Stars](https://img.shields.io/github/stars/spectrelang/spectre)

This repository contains the compiler for the Spectre Programming Language.

Spectre is a statically typed, design-by-contract language aiming to offer low-level control, no hidden allocations, and compile-time-enforced correctness. The compiler is written entirely in Spectre itself.

Resources:

- [Documentation](https://spectre-docs.pages.dev)
- [Devlog](https://spectrelang.org/log/devlog)
- [Matrix](https://matrix.to/#/%23spectrelang:matrix.org)
- [Discord](https://discord.gg/aCnnVwAUWU)
- [Visual Studio Code Extension](https://marketplace.visualstudio.com/items?itemName=NavidM.spectre-lang)
- [Neovim Extension](https://github.com/spectrelang/spectre-nvim)
- [Language Server](https://github.com/spectrelang/spectre-ls)
- [Debugger](https://github.com/spectrelang/phantom)

## Installation

Prerequisite: cmake

Run the following in the terminal (Linux/Mac):

```
curl https://spectrelang.org/get.sh | sh
```

Or in Windows PowerShell:

```
irm https://spectrelang.org/get.ps1 | iex
```

Then run to confirm installation:

```
spectre -v
```

Linux and Windows targets use a self-contained freestanding runtime by
default: no system libc, no CRT, and no Linux sysroot. Hosted builds remain
available with `-runtime:hosted`.

```
spectre app.sx -runtime:freestanding -o app
spectre app.sx -target:windows.x86_64 -runtime:freestanding -o app.exe
```

See `docs/FREESTANDING_RUNTIME.adoc` for runtime details and Windows import
synthesis.

The compiler is tested under MacOS aarch64 and Linux x86_64, a Windows implementation exists, though is currently not as maintained.

An example program:

```spectre
val stdio = use("std/stdio")
val str   = use("std/string")

pub fn main() void = {
    val xs: [_]str.StringView = [
        "hello", "world", "this", "is", "a", "test"
    ]
    for x in xs {
        trust stdio.print("{s}\n", {x})
    }
}
```

## License

GPL-3.0-only - (C) Navid M - 2026
