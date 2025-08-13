export VERSION := "0.4.3"

build version=VERSION:
    docker build -t geonames-fst:{{version}} -f docker/build.Dockerfile .

duui version=VERSION:
    @just build {{version}}
    docker build --build-arg VERSION={{version}} -t docker.texttechnologylab.org/duui-geonames-fst/base:{{version}} -f docker/duui.Dockerfile .
    docker tag docker.texttechnologylab.org/duui-geonames-fst/base:{{version}} docker.texttechnologylab.org/duui-geonames-fst/base:latest
    docker push docker.texttechnologylab.org/duui-geonames-fst/base:{{version}}
    docker push docker.texttechnologylab.org/duui-geonames-fst/base:latest
