FROM mcr.microsoft.com/devcontainers/rust:1-1-bullseye


RUN apt-get update \
    && apt-get install libudev-dev \
    && apt-get clean -y && rm -rf /var/lib/apt/lists/* /tmp/library-scripts