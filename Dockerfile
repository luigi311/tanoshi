FROM rust:latest AS builder

WORKDIR /app

RUN apt install -y libssl-dev git curl
RUN curl -fsSL https://deb.nodesource.com/setup_16.x | bash -
RUN apt upgrade -y && apt-get install -y nodejs libssl-dev libarchive-dev
RUN npm install -g yarn

COPY . .

RUN cargo install wasm-bindgen-cli wasm-pack
RUN cd /app/tanoshi/tanoshi-web && yarn install && yarn build
RUN cd /app/tanoshi && cargo build -p tanoshi --release

FROM debian:bullseye-slim

WORKDIR /app

COPY --from=builder /app/tanoshi/target/release/tanoshi .
RUN chmod +x tanoshi

RUN apt update && apt upgrade -y && apt install --reinstall -y ca-certificates libarchive-dev

ENV RUST_LOG=tanoshi=info
ENV TANOSHI_HOME=/tanoshi

EXPOSE 80

ENTRYPOINT ["/app/tanoshi"]