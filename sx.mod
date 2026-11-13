entry "./src/sxc.sx"
version "v0.0.17"
spectre_version "v0.0.16"

build release {
    flags "-release -o spectre-release"
    env {
        GIT_HASH=release-test-build
    }
}

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
