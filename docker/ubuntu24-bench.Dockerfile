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
    && rm -rf /var/lib/apt/lists/*

RUN curl -fsSL https://sh.rustup.rs | sh -s -- -y --profile minimal

ENV PATH="/root/.cargo/bin:${PATH}"

RUN rustup default stable

WORKDIR /work
