entry "./src/sxc.sx"
version "v0.0.7"

build prod {
    action "./build.sh"
    // flags "--release"
    // output "./spectre"
}

build dev {
    output "./spectre-dev"
}
