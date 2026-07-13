ARG NGINX_VERSION=1.31.2

FROM ubuntu:24.04 AS nginx-builder
ARG NGINX_VERSION
ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        build-essential \
        ca-certificates \
        curl \
        libssl-dev \
        zlib1g-dev \
    && rm -rf /var/lib/apt/lists/*

RUN curl -fsSL "https://nginx.org/download/nginx-${NGINX_VERSION}.tar.gz" -o /tmp/nginx.tar.gz \
    && mkdir /tmp/nginx-src \
    && tar -xzf /tmp/nginx.tar.gz -C /tmp/nginx-src --strip-components=1 \
    && cd /tmp/nginx-src \
    && ./configure \
        --prefix=/opt/nginx \
        --with-http_ssl_module \
        --with-http_v2_module \
        --with-stream \
        --with-threads \
        --without-http_rewrite_module \
        --with-cc-opt='-O3 -fno-plt' \
        --with-ld-opt='-Wl,-O1,--as-needed' \
    && make -j"$(nproc)" \
    && make install

FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive
ENV PATH="/opt/nginx/sbin:${PATH}"

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        openssl \
        zlib1g \
    && rm -rf /var/lib/apt/lists/*

COPY --from=nginx-builder /opt/nginx /opt/nginx
COPY proxysss /usr/local/bin/proxysss
RUN chmod 0755 /usr/local/bin/proxysss

WORKDIR /work
