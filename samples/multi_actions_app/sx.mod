entry "app.sx"
version "v0.0.1"
spectre_version "v0.0.17"

dep "https://example.com/multi-actions-dep.git" {
    build_actions {
        ["echo", "dep-first:list-form"],
        "echo dep-second:free-form"
    }
}

build release {
    flags ["-release"]
    output "./multi_actions_app"
    posix {
        actions {
            ["sh", "scripts/prepare.sh", "release", "$OUT/prepare_release.txt"],
            ["sh", "scripts/build.sh"]
        }
    }
    windows {
        actions {
            ["cmd", "/c", "scripts\prepare.cmd", "release", "$OUT/prepare_release.txt"],
            ["cmd", "/c", "scripts\build.cmd"]
        }
    }
}

build free {
    flags ["-release"]
    output "./multi_actions_app_free"
    posix {
        actions {
            "sh scripts/prepare.sh free $OUT/prepare_free.txt && sh scripts/build.sh"
        }
    }
    windows {
        actions {
            "cmd /c scripts\prepare.cmd free $OUT/prepare_free.txt && cmd /c scripts\build.cmd"
        }
    }
}

build artifacts {
    posix {
        actions {
            ["sh", "scripts/mark.sh", "$OUT/artifact.txt", "cross-target"],
            "sh scripts/mark.sh $OUT/artifact_free.txt cross-target-free"
        }
    }
    windows {
        actions {
            ["cmd", "/c", "scripts\mark.cmd", "$OUT/artifact.txt", "cross-target"],
            "cmd /c scripts\mark.cmd $OUT/artifact_free.txt cross-target-free"
        }
    }
}
