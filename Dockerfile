# Rust as the base image
FROM rust:latest as build

# Create a new empty shell project
RUN USER=root cargo new --bin danielek
WORKDIR /danielek

# Copy our manifests
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml

# Build only the dependencies to cache them
RUN cargo build --release && rm src/*.rs

# Copy the source code
COPY ./src ./src

# Build for release.
RUN rm ./target/release/deps/danielek*
RUN cargo build --release

# The final base image
FROM debian:buster-slim
RUN apt update && apt install -y libssl-dev ca-certificates
ENV RUST_BACKTRACE=1

# Copy from the previous build
COPY --from=build /danielek/target/release/danielek /usr/src/danielek

CMD ["/usr/src/danielek"]