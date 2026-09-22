FROM rust:1.96-bookworm

RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY . .

RUN cargo fetch --locked \
    && cargo test --offline --no-run --test committed_agent_rename \
    && cargo test --offline --no-run -p zeroclaw-gateway --lib

CMD ["bash"]
