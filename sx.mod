entry "./src/sxc.sx"
version "v0.0.17"
spectre_version "v0.0.17"

build release {
    posix {
        flags "-release -o spectre-release"
    }
    windows {
        flags "-release -o spectre-release.exe"
    }
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
    posix {
        output "./spectre-dev"
    }
    windows {
        output "./spectre-dev.exe"
    }
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
