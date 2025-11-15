FROM rust:1.91 AS chef
RUN cargo install cargo-chef
WORKDIR /usr/src/rememberd

FROM chef AS planner
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /usr/src/rememberd/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release

FROM debian:bookworm-slim
WORKDIR /app
COPY --from=builder /usr/src/rememberd/target/release/rememberd /app/rememberd
COPY templates ./templates
EXPOSE 8080
CMD ["/app/rememberd"]
