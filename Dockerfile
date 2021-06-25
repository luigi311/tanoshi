FROM debian:bullseye-slim

WORKDIR /app

RUN apt update && apt upgrade -y && apt install --reinstall -y ca-certificates curl

RUN curl -sL https://github.com/faldez/tanoshi/releases/download/v0.23.0/tanoshi-linux -o tanoshi
RUN chmod +x tanoshi

ENV RUST_LOG=info

EXPOSE 80

CMD ["/app/tanoshi", "--config", "/tanoshi/config.yml"]