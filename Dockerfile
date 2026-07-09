FROM ghcr.io/luigi311/tanoshi-builder:sha-6e8cc9c AS base

# Backend builder
FROM base AS builder

# The builder image runs as the non-root 'ubuntu' user; without --chown the
# copied sources are root-owned and trunk/cargo can't write build output.
COPY --chown=ubuntu:ubuntu . .

RUN cd crates/tanoshi-web && trunk build --release

RUN cargo build -p tanoshi --release



FROM ubuntu:26.04 AS runtime

RUN apt update && apt upgrade -y && apt install --reinstall -y tini ca-certificates libssl3 libxml2-dev

WORKDIR /app

COPY --from=builder /app/target/release/tanoshi .
RUN chmod +x tanoshi

ENV PORT=80
ENV TANOSHI_LOG=info
ENV TANOSHI_HOME=/tanoshi

EXPOSE $PORT

ENTRYPOINT ["/bin/tini", "--", "/app/tanoshi"]
