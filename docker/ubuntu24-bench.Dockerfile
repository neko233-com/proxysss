ARG DOCKER_CLI_VERSION=29
FROM docker:${DOCKER_CLI_VERSION}-cli AS docker-cli

FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        curl \
        build-essential \
        pkg-config \
        make \
        gcc \
        g++ \
        perl \
        python3 \
        openssl \
        libssl-dev \
        zlib1g-dev \
        xz-utils \
        git \
        golang-go \
    && rm -rf /var/lib/apt/lists/*

RUN curl -fsSL https://sh.rustup.rs | sh -s -- -y --profile minimal

ENV PATH="/root/.cargo/bin:${PATH}"

RUN rustup default stable

COPY --from=docker-cli /usr/local/bin/docker /usr/local/bin/docker

WORKDIR /work
