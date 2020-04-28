FROM rust:1.43-buster

ARG TOKEN_SECRET_KEY
ARG STATIC_FILES_PATH
ARG DATABASE_URL

ENV TOKEN_SECRET_KEY ${TOKEN_SECRET_KEY}
ENV STATIC_FILES_PATH ${STATIC_FILES_PATH}
ENV DATABASE_URL ${DATABASE_URL}

RUN /bin/sh -c env

RUN apt update && apt install -y git curl
RUN curl -sS https://dl.yarnpkg.com/debian/pubkey.gpg | apt-key add -
RUN echo "deb https://dl.yarnpkg.com/debian/ stable main" | tee /etc/apt/sources.list.d/yarn.list
RUN apt-get update && apt install -y yarn

WORKDIR /usr/src
RUN git clone https://github.com/fadhlika/tanoshi-web /usr/src/tanoshi-web
RUN git clone https://github.com/fadhlika/tanoshi /usr/src/tanoshi
RUN mkdir -p /tanoshi

WORKDIR /usr/src/tanoshi-web
RUN yarn install
RUN yarn build
RUN mkdir -p $STATIC_FILES_PATH && cp -R /usr/src/tanoshi-web/dist /tanoshi/
RUN ls -l /tanoshi

WORKDIR /usr/src/tanoshi
RUN cargo build --release
RUN ls -l target/release
RUN cp /usr/src/tanoshi/target/release/tanoshi /usr/local/bin/tanoshi

EXPOSE 3030
CMD ["tanoshi"]
