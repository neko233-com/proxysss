FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        nginx-full \
        openssl \
    && rm -rf /var/lib/apt/lists/*

COPY proxysss /usr/local/bin/proxysss
RUN chmod 0755 /usr/local/bin/proxysss

WORKDIR /work
