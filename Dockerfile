FROM rust:alpine as builder
# Setting TZ makes "make" build the correct time (at least for local builds) into the executable
ARG TZ=Europe/Stockholm
ENV TZ=${TZ}
#ENV RUSTFLAGS="-C target-cpu=native -C link-self-contained=yes"
ENV SQLX_OFFLINE="true"

RUN apk add --no-cache --update \
  alpine-sdk \
  musl-dev \
  tzdata \
  && rm -rf /var/cache/apk/*

WORKDIR /app
COPY . .
# It seems that since the target directory is mounted as cache, it won't be available
# in the next stage, and hence we need to move the binary to another location in order to be
# able to copy it in the next stage
RUN --mount=type=cache,target=/usr/local/cargo/registry \
  --mount=type=cache,target=/app/target \
  cargo build --features=bundled --release --bin rlunch \
  && mv /app/target/release/rlunch /tmp/

FROM alpine:latest
LABEL maintainer="Odd E. Ebbesen <oddebb@gmail.com>"
ARG TZ=Europe/Stockholm
ENV TZ=${TZ}
RUN apk add --no-cache --update \
  ca-certificates \
  tzdata \
  && \
  rm -rf /var/cache/apk/*
RUN adduser -D -u 1000 lunchsrv
COPY --from=builder /tmp/rlunch /usr/local/bin/rlunch
RUN chown lunchsrv /usr/local/bin/rlunch && chmod 555 /usr/local/bin/rlunch
USER lunchsrv
CMD ["rlunch"]
