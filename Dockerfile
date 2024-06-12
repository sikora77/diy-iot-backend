FROM rust:alpine
LABEL authors="sikora"

WORKDIR /usr/src/diy-iot
COPY . .
# RUN cargo install --path .
RUN apk add build-base
RUN apk add openssl-dev pkgconfig perl
RUN apk add postgresql postgresql-dev postgresql-contrib
RUN rustup override set nightly
ENV RUSTFLAGS='-C target-feature=-crt-static'
RUN echo $RUSTFLAGS
RUN cargo +nightly build --release
# EXPOSE 8000

CMD ["ash"]