entry "sample_fps.sx"
version "v0.0.1"
spectre_version "v0.0.12"

build release {
    flags "--release"
    output "./samples"
}

build dev {
    flags "--alt --inc-path=/usr/include/SDL2/"
    output "./samples-dev"
}

build qbe {
    flags "--inc-path=/usr/include/SDL2/"
    output "./samples-dev"
}

dep "https://github.com/spectrelang/ssdl2.git"
