FROM docker.io/rust:1.85 AS build

RUN apt-get update && apt-get install -y cmake && rm -rf /var/lib/apt/lists/*

COPY src /build/src
COPY Cargo.toml /build/Cargo.toml
COPY Cargo.lock /build/Cargo.lock

WORKDIR /build

RUN ["cargo", "build", "--release"]
RUN chmod +x /build/target/release/geonames-fst

### Production ###

FROM cgr.dev/chainguard/glibc-dynamic:latest AS prod
# FROM debian:bookworm-slim AS prod

WORKDIR /app

COPY --from=build /build/target/release/geonames-fst /app/geonames-fst
COPY data/geonames/DE.txt /app/data/geonames/
COPY data/alternateNames/DE.txt /app/data/alternateNames/

ENTRYPOINT [ "/app/geonames-fst", "/app/data/geonames/" , "--alternate", "/app/data/alternateNames/", "--host", "0.0.0.0", "--port", "8000" ]
