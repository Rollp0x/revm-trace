# syntax=docker/dockerfile:1

# This Dockerfile verifies that our code can compile and run without OpenSSL
# by using Alpine Linux which doesn't include OpenSSL by default.
# If the build succeeds, it confirms that the rustls-tls feature works correctly.

ARG RUST_VERSION=1.81.0
ARG APP_NAME=revm-trace

FROM rust:${RUST_VERSION}-alpine AS build
ARG APP_NAME
WORKDIR /app

# Install minimal host build dependencies.
# Note: we intentionally DO NOT install openssl-dev here
# to verify that our code can compile without OpenSSL.
RUN apk add --no-cache clang lld musl-dev git

# Copy the project files
COPY . .

# # First build the example to verify it compiles without OpenSSL
RUN cargo build --release --no-default-features --example transfer_eth_insufficient_balance --features rustls-tls && \
cp ./target/release/examples/transfer_eth_insufficient_balance /bin/server


FROM alpine:3.18 AS final

# Create a non-privileged user that the app will run under.
# See https://docs.docker.com/go/dockerfile-user-best-practices/
ARG UID=10001
RUN adduser \
    --disabled-password \
    --gecos "" \
    --home "/nonexistent" \
    --shell "/sbin/nologin" \
    --no-create-home \
    --uid "${UID}" \
    appuser
USER appuser

# Copy the executable from the "build" stage.
COPY --from=build /bin/server /bin/

# What the container should run when it is started.
CMD ["/bin/server"]
