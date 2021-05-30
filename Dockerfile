FROM debian:buster AS builder

WORKDIR /app

RUN curl https://sh.rustup.rs -sSf | sh

RUN apt update
RUN apt install -y \
    curl \
    git \
    build-essential \
    openssl \
    libssl-dev \
    pkg-config

RUN curl https://sh.rustup.rs -sSf | bash -s -- -y

ENV PATH="/root/.cargo/bin:${PATH}"

RUN curl -fsSL https://deb.nodesource.com/setup_16.x | bash -
RUN apt-get install -y nodejs 
RUN npm install -g yarn

RUN git clone --depth 1 --branch v0.22.0 https://github.com/faldez/tanoshi.git

WORKDIR /app/tanoshi/tanoshi-web

RUN yarn install
RUN cargo install wasm-bindgen-cli wasm-pack
RUN yarn build

WORKDIR /app/tanoshi

RUN cargo build --release

FROM debian:buster-slim AS tanoshi

RUN mkdir /tanoshi
RUN mkdir /config

WORKDIR /app

RUN apt update
RUN apt install -y openssl libssl-dev

COPY --from=builder /app/tanoshi/target/release/tanoshi tanoshi

CMD [ "/app/tanoshi", "--config", "/config/config.yml" ]