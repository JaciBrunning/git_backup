FROM rust AS builder

# Dependency Caching
WORKDIR /build
RUN USER=root cargo new --bin git_backup
WORKDIR /build/git_backup
COPY ./Cargo.toml ./Cargo.toml
RUN cargo build --release
RUN rm src/*.rs
RUN rm ./target/release/deps/git_backup*

# Build the real thing
COPY . .
RUN cargo build --release

# Final production image without rust build bootstrapping
FROM debian:buster-slim
RUN apt-get update \
    && apt-get install -y openssl ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /work
COPY --from=builder /build/git_backup/target/release/git_backup /bin/git_backup
ENV RUST_LOG=info

CMD [ "git_backup" ]