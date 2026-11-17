entry "./src/sxc.sx"
version "v0.0.18"
spectre_version "v0.0.17"

var {
    DEFAULT="dev-build"
    EXT=""
    windows {
        EXT=".exe"
    }
}

build release {
    flags "-release -o spectre-release$EXT"
    env {
        GIT_HASH=$((git rev-parse --short HEAD))
    }
}

build loud {
    output "./spectre-dev$EXT"
    flags "-l"
    env {
        GIT_HASH="$DEFAULT"
    }
}

build dev {
    output "./spectre-dev$EXT"
    env {
        GIT_HASH="$DEFAULT"
    }
}

build quick {
    flags "-alt"
    output "./spectre-dev$EXT"
    env {
        GIT_HASH="$DEFAULT-alt"
    }
}
