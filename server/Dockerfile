FROM rust:1-slim AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
RUN --mount=type=bind,from=builder,source=/app/target/release/server,target=/tmp/server cp /tmp/server .
EXPOSE 3001
CMD ["./server"]
