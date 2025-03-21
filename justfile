export VERSION := "0.1.0"


build version=VERSION:
    docker build -t geonames-fst:{{version}} -f docker/build.Dockerfile .

duui version=VERSION:
    @just build {{version}}
    docker build --build-arg VERSION={{version}} -t docker.texttechnologylab.org/fst/geonames:{{version}} -f docker/duui.Dockerfile .
