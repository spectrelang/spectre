entry "./src/sxc.sx"

build prod {
    action "./build.sh"
    // flags "--release"
    // output "./spectre"
}

build dev {
    output "./spectre-dev"
}
