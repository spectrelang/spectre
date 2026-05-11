entry "./src/sxc.sx"
version "v0.0.9"

build prod {
    action "./fw/build.sh"
    // flags "--release"
    // output "./spectre"
}

build dev {
    output "./spectre-dev"
}

build quick {
    flags "--alt"
    output "./spectre-dev"
}
