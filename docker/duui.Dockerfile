ARG VERSION="latest"
FROM geonames-fst:$VERSION AS builder

RUN ["cargo", "build", "--release", "--no-default-features", "--features", "duui"]
RUN chmod +x /build/target/release/geonames-fst
