entry "./src/sxc.sx"
version "v0.0.15"
spectre_version "v0.0.15"

build prod {
    action "./fw/build.sh"
}

build loud {
    flags "-l"
}

build dev {
    output "./spectre-dev"
    env {
        GIT_HASH=dev-build-unknown
    }
}

build quick {
    flags "-alt"
    output "./spectre-dev"
}
