ARG BENCH_BASE=proxysss-ubuntu24-bench:latest
FROM docker:cli AS docker_cli
FROM ${BENCH_BASE}

# The benchmark base already provides bash + Go. Copy the statically linked
# official Docker CLI and Buildx plugin so release gates never install packages
# at run time and remain usable during distro-mirror outages.
COPY --from=docker_cli /usr/local/bin/docker /usr/local/bin/docker
COPY --from=docker_cli /usr/local/libexec/docker/cli-plugins/docker-buildx \
  /usr/local/libexec/docker/cli-plugins/docker-buildx

RUN chmod 0755 /usr/local/bin/docker \
  /usr/local/libexec/docker/cli-plugins/docker-buildx

WORKDIR /work
