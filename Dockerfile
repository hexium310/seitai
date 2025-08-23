# syntax = docker/dockerfile:1
FROM rust:1.89-slim-bookworm AS runtime
WORKDIR /usr/src/myapp
RUN --mount=type=cache,id=api:/var/cache/apt,target=/var/cache/apt \
    --mount=type=cache,id=api:/var/lib/apt/lists,target=/var/lib/apt/lists \
    apt-get update && apt-get install --no-install-recommends -y \
    libopus-dev \
    libssl-dev \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

FROM runtime AS development

FROM runtime AS builder
RUN --mount=type=bind,source=crates,target=crates \
    --mount=type=bind,source=restarter,target=restarter \
    --mount=type=bind,source=seitai,target=seitai \
    --mount=type=bind,source=Cargo.toml,target=Cargo.toml \
    --mount=type=bind,source=Cargo.lock,target=Cargo.lock \
    --mount=type=cache,target=/usr/src/myapp/target \
    cargo build --release --workspace \
    && cp target/release/restarter /restarter \
    && cp target/release/seitai /seitai \
    && bash -c 'mkdir -p /{seitai,restarter}_libs/lib{,64}' \
    && ldd /seitai | awk '/=>/{ print $3 }' | xargs cp --dereference --target-directory=/seitai_libs/lib \
    && ldd /restarter | awk '/=>/{ print $3 }' | xargs cp --dereference --target-directory=/restarter_libs/lib

FROM scratch AS restarter
LABEL io.github.hexium310.seitai.app=restarter
LABEL org.opencontainers.image.source=https://github.com/hexium310/seitai
COPY --link --from=builder /etc/ssl/certs/ /etc/ssl/certs/
COPY --link --from=builder /restarter_libs/lib/* /lib/
COPY --link --from=builder /lib64/ld-linux-x86-64.so* /lib64/
COPY --link --from=builder /restarter /
CMD ["/restarter"]

FROM scratch AS seitai
LABEL io.github.hexium310.seitai.app=seitai
LABEL org.opencontainers.image.source=https://github.com/hexium310/seitai
COPY --link --from=builder /etc/ssl/certs/ /etc/ssl/certs/
COPY --link --from=builder /seitai_libs/lib/* /lib/
COPY --link --from=builder /lib64/ld-linux-x86-64.so* /lib64/
COPY --link --from=builder /seitai /
CMD ["/seitai"]
