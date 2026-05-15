# Spectre

![License](https://img.shields.io/badge/license-GPLv3-red)
![Status](https://img.shields.io/badge/status-alpha-red)
![Stars](https://img.shields.io/github/stars/spectrelang/spectre)

This repository contains the compiler for the Spectre Programming Language.

Spectre is a statically typed, design-by-contract language aiming to offer low-level control in combination with explicit correctness. The compiler is written entirely in Spectre itself.

Resources:

- [Documentation](https://spectre-docs.pages.dev)
- [Playground](https://spectrelang.org/playground)
- [Devlog](https://spectrelang.org/log/devlog)
- [Matrix](https://matrix.to/#/%23spectrelang:matrix.org)
- [Discord](https://discord.gg/aCnnVwAUWU)
- [Visual Studio Code Extension](https://marketplace.visualstudio.com/items?itemName=NavidM.spectre-lang)
- [Neovim Extension](https://github.com/spectrelang/spectre-nvim)
- [Language Server](https://github.com/spectrelang/spectre-ls)

## Installation

Prerequisite: cmake

Run the following in the terminal:

```
curl https://spectrelang.org/get.sh | sh
```

Then run to confirm installation:

```
spectre -v
```

The compiler is tested under MacOS aarch64 and Linux x86_64, a Windows implementation exists, though is currently not as well-maintained.

An example program:

```spectre
val stdio = use("std/stdio")

pub fn main() void = {
    val xs = ["hello", "world", "this", "is", "a", "test"]
    for x in xs {
        trust stdio.print("{s}\n", {x})
    }
}
```

## License

GPL-3.0-only - (C) Navid M - 2026
