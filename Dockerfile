# syntax = docker/dockerfile:1
FROM rust:1.82-slim-bookworm AS runtime
RUN --mount=type=cache,id=api:/var/cache/apt,target=/var/cache/apt \
    --mount=type=cache,id=api:/var/lib/apt/lists,target=/var/lib/apt/lists \
    apt-get update && apt-get install --no-install-recommends -y \
    libopus-dev \
    libssl-dev \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

FROM runtime AS development
WORKDIR /usr/src/myapp

FROM runtime AS builder
WORKDIR /usr/src/myapp
COPY . .
RUN --mount=type=cache,target=/usr/src/myapp/target \
    cargo build --release --workspace \
    && cp target/release/restarter /restarter \
    && cp target/release/seitai /seitai

FROM scratch AS restarter
LABEL io.github.hexium310.seitai.app=restarter
LABEL org.opencontainers.image.source=https://github.com/hexium310/seitai
COPY --from=runtime /etc/ssl/certs/ /etc/ssl/certs/
COPY --from=runtime /lib/x86_64-linux-gnu/libc.so* /lib/x86_64-linux-gnu/
COPY --from=runtime /lib/x86_64-linux-gnu/libcrypto.so* /lib/x86_64-linux-gnu/
COPY --from=runtime /lib/x86_64-linux-gnu/libgcc_s.so* /lib/x86_64-linux-gnu/
COPY --from=runtime /lib/x86_64-linux-gnu/libm.so* /lib/x86_64-linux-gnu/
COPY --from=runtime /lib/x86_64-linux-gnu/libssl.so* /lib/x86_64-linux-gnu/
COPY --from=runtime /lib64/ld-linux-x86-64.so* /lib64/
COPY --from=builder /restarter /
CMD ["/restarter"]

FROM scratch AS seitai
LABEL io.github.hexium310.seitai.app=seitai
LABEL org.opencontainers.image.source=https://github.com/hexium310/seitai
COPY --from=runtime /etc/ssl/certs/ /etc/ssl/certs/
COPY --from=runtime /lib/x86_64-linux-gnu/libc.so* /lib/x86_64-linux-gnu/
COPY --from=runtime /lib/x86_64-linux-gnu/libcrypto.so* /lib/x86_64-linux-gnu/
COPY --from=runtime /lib/x86_64-linux-gnu/libgcc_s.so* /lib/x86_64-linux-gnu/
COPY --from=runtime /lib/x86_64-linux-gnu/libm.so* /lib/x86_64-linux-gnu/
COPY --from=runtime /lib/x86_64-linux-gnu/libopus.so* /lib/x86_64-linux-gnu/
COPY --from=runtime /lib/x86_64-linux-gnu/libssl.so* /lib/x86_64-linux-gnu/
COPY --from=runtime /lib64/ld-linux-x86-64.so* /lib64/
COPY --from=builder /seitai /
CMD ["/seitai"]
