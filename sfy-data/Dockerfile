FROM alpine:latest as BUILD

RUN apk update && \
    apk upgrade && \
    apk add curl git build-base cmake openssl-dev

RUN curl https://sh.rustup.rs -sSf | sh -s -- --profile minimal --default-toolchain nightly -y
ENV PATH="$PATH:/root/.cargo/bin"

WORKDIR /work

ADD . sfy-data/

WORKDIR /work/sfy-data

RUN cargo build --release --no-default-features --features postgres

FROM registry.met.no/baseimg/alpine:latest

RUN apk update && \
    apk upgrade && \
    apk add build-base openssl bash

WORKDIR /sfy
COPY --from=0 /work/sfy-data/target/release/sfy-data ./

ADD sfy-data.toml /sfy/sfy-data.toml

RUN addgroup -S sfy && adduser -S sfy -G sfy -s /bin/false
USER sfy:sfy

CMD /sfy/sfy-data
