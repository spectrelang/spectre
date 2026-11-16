entry "./src/sxc.sx"
version "v0.0.17"
spectre_version "v0.0.17"

var {
    EXT=""
    windows {
        EXT=".exe"
    }
}

build release {
    flags "-release -o spectre-release$EXT"
    env {
        GIT_HASH=release-test-build
    }
}

build loud {
    flags "-l"
    env {
        GIT_HASH=dev-build
    }
}

build dev {
    output "./spectre-dev$EXT"
    env {
        GIT_HASH=dev-build
    }
}

build quick {
    flags "-alt"
    output "./spectre-dev$EXT"
    env {
        GIT_HASH=dev-build-quick
    }
}
