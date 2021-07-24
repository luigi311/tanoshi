FROM rust:latest AS builder

WORKDIR /app

RUN apt install -y curl
RUN curl -fsSL https://deb.nodesource.com/setup_16.x | bash -
RUN apt upgrade -y && \
    apt-get install -y \
    nodejs \
    libssl-dev \
    libarchive-dev \
    build-essential \
    cmake \
    llvm \
    clang \
    libicu-dev \
    nettle-dev \
    libacl1-dev \
    liblzma-dev \
    libzstd-dev \
    liblz4-dev \
    libbz2-dev \
    zlib1g-dev \
    libxml2-dev
RUN npm install -g yarn
RUN cargo install wasm-pack

COPY . .

RUN yarn --cwd tanoshi-web && yarn --cwd tanoshi-web build
RUN cargo build -p tanoshi --release

FROM debian:buster-slim

WORKDIR /app

COPY --from=builder /app/target/release/tanoshi .
RUN chmod +x tanoshi

RUN apt update && apt upgrade -y && apt install --reinstall -y ca-certificates

ENV TANOSHI_LOG=info
ENV TANOSHI_HOME=/tanoshi

EXPOSE 80

ENTRYPOINT ["/app/tanoshi"]