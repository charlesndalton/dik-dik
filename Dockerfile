FROM docker.io/clux/muslrust:1.59.0 as cargo-build
WORKDIR /app
ADD . /app
RUN env CARGO_PROFILE_RELEASE_DEBUG=1 cargo build --target x86_64-unknown-linux-musl --release


FROM docker.io/alpine:latest

RUN apk add --no-cache tini

COPY --from=cargo-build /app/target/x86_64-unknown-linux-musl/release/dik-dik /

ENV RUST_LOG=INFO
CMD ["./dik-dik"]
