# Built from this folder: docker build -t greybel-editor-service .
FROM rust:1.93-slim-bookworm AS build
WORKDIR /service
COPY Cargo.toml Cargo.lock ./
COPY migrations migrations
COPY src src
RUN cargo build --release --locked

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=build /service/target/release/greybel-editor-service /usr/local/bin/greybel-editor-service
USER nobody
ENV PORT=8080
EXPOSE 8080
CMD ["greybel-editor-service"]
