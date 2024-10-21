FROM rust:alpine as builder
# Setting TZ makes "make" build the correct time (at least for local builds) into the executable
ARG ARG_TZ=Europe/Stockholm
ENV TZ=${ARG_TZ}
#ENV RUSTFLAGS="-C target-cpu=native -C link-self-contained=yes"
ENV SQLX_OFFLINE="true"

RUN apk add --no-cache --update \
  alpine-sdk \
  musl-dev \
  && rm -rf /var/cache/apk/*

WORKDIR /app
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
  --mount=type=cache,target=/usr/local/src/rlunch/target \
  cargo build --features=bundled --release --bin rlunch

FROM alpine:latest
RUN apk add --no-cache --update \
  ca-certificates \
  tzdata \
  && \
  rm -rf /var/cache/apk/*
RUN adduser -D -u 1000 lunchsrv
COPY --from=builder /app/target/release/rlunch /usr/local/bin/rlunch
RUN chown lunchsrv /usr/local/bin/rlunch && chmod 555 /usr/local/bin/rlunch
USER lunchsrv
CMD ["rlunch"]
