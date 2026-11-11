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
- [YouTube](https://www.youtube.com/@spectrelang)
- [Visual Studio Code Extension](https://marketplace.visualstudio.com/items?itemName=NavidM.spectre-lang)
- [Neovim Extension](https://github.com/spectrelang/spectre-nvim)
- [Language Server](https://github.com/spectrelang/spectre-ls)
- [Debugger](https://github.com/spectrelang/phantom)
- [Libraries and Ecosystem](https://spectrelang.org/spectreforge)

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

> [!NOTE]
> Linux and Windows targets use a self-contained freestanding runtime by
> default, meaning no system libc, CRT, or Linux sysroot. Hosted builds remain
> available with `-runtime:hosted`. Cross compilation from Linux to Windows, for example, is as simple as:
>
> ```
> spectre app.sx -target:windows.x86_64 -o app.exe
> ```
>
> This will produce `app.exe`, a 64-bit executable for Windows.

## Example

An example program is as follows:

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

GPL-3.0-only - (C) Navid M. - 2026
