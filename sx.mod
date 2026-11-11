entry "./src/sxc.sx"
version "v0.0.16"
spectre_version "v0.0.16"

build prod {
    action "./fw/build.sh"
}

build loud {
    flags "-l"
    env {
        GIT_HASH=dev-build
    }
}

build dev {
    output "./spectre-dev"
    env {
        GIT_HASH=dev-build
    }
}

build quick {
    flags "-alt"
    output "./spectre-dev"
    env {
        GIT_HASH=dev-build-quick
    }
}
