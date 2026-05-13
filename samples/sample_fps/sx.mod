entry "sample_fps.sx"
version "v0.0.1"

build release {
    flags "--release"
    output "./samples"
}

build dev {
    flags "--alt --inc-path=/usr/include/SDL2/"
    output "./samples-dev"
}

build qbe {
    output "./samples-dev"
}

dep "https://github.com/spectrelang/ssdl2.git"
