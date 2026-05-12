./spectre-dev ./src/sxc.sx --emit-ssa --src-target=linux > bootstrap/sxc.ssa
./spectre-dev ./src/sxc.sx -b --emit-alt --src-target=linux > bootstrap/sxc.c
./spectre-dev ./src/sxc.sx --emit-ssa --src-target=darwin > bootstrap/sxc_darwin.ssa
./spectre-dev ./src/sxc.sx -b --emit-alt --src-target=darwin > bootstrap/sxc_darwin.c
./spectre-dev ./src/sxc.sx -b --emit-alt --src-target=windows > bootstrap/sxcw.c
