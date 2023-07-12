FROM rust:1.70-slim-bookworm

RUN apt-get update && apt-get install -y \
      ffmpeg \
      libopus-dev \
      libssl-dev \
      pkg-config \
      && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/myapp
