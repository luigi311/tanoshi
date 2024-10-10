FROM ghcr.io/luigi311/tanoshi-builder:sha-6886658-slim AS base

# Backend builder
FROM base AS builder

COPY . .

RUN cd crates/tanoshi-web && trunk build --release

RUN cargo build -p tanoshi --release



FROM debian:bookworm-slim AS runtime

RUN apt update && apt upgrade -y && apt install --reinstall -y ca-certificates libssl3 libxml2

WORKDIR /app

COPY --from=builder /app/target/release/tanoshi .
RUN chmod +x tanoshi

ENV PORT=80
ENV TANOSHI_LOG=info
ENV TANOSHI_HOME=/tanoshi

EXPOSE $PORT

ENTRYPOINT ["/app/tanoshi"]
