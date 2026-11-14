entry "app.sx"
version "v0.0.1"
spectre_version "v0.0.17"

var {
    CC=gcc // OUT is reserved; it always resolves to {project-root}/.sx/.
    SOME_OTHER_VAR=something
}

build release {
    flags ["-release", "-o", "some_app", "-link-obj:$OUT/tiny-json.o", "-inc-path:lib/tiny-json"]
}

build dev {
    flags ["-o", "some_app", "-link-obj:$OUT/tiny-json.o", "-inc-path:lib/tiny-json"]
}

dep "https://github.com/rafagafe/tiny-json.git" {
    build_action ["$CC", "-c", "tiny-json.c", "-std=c99", "-O2", "-Dstatic=", "-Dinline=", "-o", "$OUT/tiny-json.o"]
}
