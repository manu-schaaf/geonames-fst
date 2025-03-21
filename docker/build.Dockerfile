FROM rust:latest AS builder

COPY src /build/src
COPY Cargo.toml /build/Cargo.toml
COPY Cargo.lock /build/Cargo.lock

WORKDIR /build

RUN ["cargo", "build", "--release"]
RUN chmod +x /build/target/release/geonames-fst
