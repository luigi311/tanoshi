FROM rust:latest AS builder

WORKDIR /app

RUN apt install -y libssl-dev git curl
RUN curl -fsSL https://deb.nodesource.com/setup_16.x | bash -
RUN apt upgrade -y && apt-get install -y nodejs libssl-dev git
RUN npm install -g yarn

RUN git clone --depth 1 --branch v0.23.0 https://github.com/faldez/tanoshi.git

RUN cargo install wasm-bindgen-cli wasm-pack
RUN cd /app/tanoshi/tanoshi-web && yarn install && yarn build
RUN cd /app/tanoshi && cargo build -p tanoshi --release

FROM debian:bullseye-slim

WORKDIR /app

COPY --from=builder /app/tanoshi/target/release/tanoshi .
RUN chmod +x tanoshi

RUN apt update && apt upgrade -y && apt install --reinstall -y ca-certificates curl

ENV RUST_LOG=tanoshi=info

EXPOSE 80

CMD ["/app/tanoshi", "--config", "/tanoshi/config.yml"]