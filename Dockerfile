FROM dhi.io/rust:1-alpine3.22-sfw-dev AS builder

#ENV RUSTFLAGS="-C target-cpu=native -C link-self-contained=yes"
ENV SQLX_OFFLINE="true"

RUN apk add --no-cache --update \
  alpine-sdk=1.1-r0 \
  && rm -rf /var/cache/apk/*

WORKDIR /build
COPY . .
# It seems that since the target directory is mounted as cache, it won't be available
# in the next stage, and hence we need to move the binary to another location in order to be
# able to copy it in the next stage
RUN \
  --mount=type=bind,source=src,target=src \
  --mount=type=bind,source=Cargo.toml,target=Cargo.toml \
  --mount=type=bind,source=Cargo.lock,target=Cargo.lock \
  --mount=type=cache,target=/build/target/ \
  --mount=type=cache,target=/usr/local/cargo/git/db \
  --mount=type=cache,target=/usr/local/cargo/registry/ \
  sfw cargo build --features=bundled --locked --release && \
  cp /build/target/release/rlunch /build/rlunch

FROM dhi.io/alpine-base:3.23
LABEL maintainer="Odd E. Ebbesen <oddebb@gmail.com>"

COPY --from=builder --chmod=555 /build/rlunch /usr/local/bin/rlunch
CMD ["rlunch"]
