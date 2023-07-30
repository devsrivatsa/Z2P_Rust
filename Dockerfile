#builder stage
FROM lukemathwalker/cargo-chef:latest-rust:1.70.0-slim AS chef
WORKDIR /app
RUN apt update && apt install lld clang -y

FROM chef as planner
COPY . .
#compute a lock-like file for our project
RUN cargo chef cook --release --bin z2p

FROM chef as builder
COPY --from=planner /app/recepie.json recepie.json
#build project dependencies but not our application
RUN cargo chef cook --release --recepie-path recepie.json
#upto this point, if the dependency tree stays the same, all layers should be cached
COPY . .
ENV SQLX_OFFLINE true
RUN cargo build --release --bin z2p

#runtime stage
FROM debian:bullseys-slim AS runtime
WORKDIR /app
RUN apt-get update -y \
    && apt-get install -y --no-install-recommends openssl ca-certificates \
    && apt-get autoremove -y \
    && apt-get clean -y \
    && rm -rf /var/lib/apt/lists/*
#copy the compiled binary from the builder environment to runtime environment
COPY --from=builder /app/target/release/z2p z2p
#we need configuration file at runtime
COPY configuration configuration
ENV APP_ENVIRONMENT production
EXPOSE 8000
#ENTRYPOINT ["./target/release/z2p"]
ENTRYPOINT ["./z2p"]