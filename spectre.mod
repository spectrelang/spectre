entry "./src/sxc.sx"

build prod {
    flags "--release"
    output "./spectre"
}

build dev {
    output "./spectre-dev"
}
