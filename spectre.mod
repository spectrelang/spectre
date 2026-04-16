entry "./src/sxc.sx"
version "v0.0.2"

build prod {
    action "./build.sh"
    // flags "--release"
    // output "./spectre"
}

build dev {
    output "./spectre-dev"
}
