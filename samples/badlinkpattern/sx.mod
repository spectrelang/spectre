entry "./repro_entry.sx"
version "v0.0.1"

build dev {
    flags "--alt=clang --show-cmd"
    output "./repro-out"
}
