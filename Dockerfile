FROM faldez/tanoshi-builder:latest AS builder

WORKDIR /app

COPY . .

RUN cd tanoshi-web && trunk build --release
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