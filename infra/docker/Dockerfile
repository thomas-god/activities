# Build stage for Svelte frontend
FROM node:26-alpine AS frontend-builder
WORKDIR /app/frontend

# Copy package.json and install dependencies
COPY client/package*.json ./
RUN npm ci

# Copy the rest of the application and build it
COPY client/vite.config.ts ./
COPY client/tsconfig.json ./
COPY client/.gitignore ./
COPY client/.npmrc ./
# COPY client/.env.production ./.env
COPY client/src/ ./src
COPY client/static/ ./static
ENV NODE_ENV=production
ENV PUBLIC_APP_URL=
RUN npm run build

FROM rust:1.96-alpine AS chef
RUN apk add --no-cache musl-dev
RUN cargo install cargo-chef --locked
WORKDIR /app/backend

# Analyse the workspace and emit a recipe describing its dependencies.
FROM chef AS planner
COPY Cargo.toml Cargo.lock ./
COPY app/ app/
COPY fit-parser/ fit-parser/
COPY fit-codegen/ fit-codegen/
RUN cargo chef prepare --recipe-path recipe.json

# Build stage for Rust backend
FROM chef AS backend-builder
COPY --from=planner /app/backend/recipe.json recipe.json
# Compile only the dependencies.
RUN cargo chef cook --release --recipe-path recipe.json
# Compile the workspace crates themselves.
COPY Cargo.toml Cargo.lock ./
COPY app/ app/
COPY fit-parser/ fit-parser/
COPY fit-codegen/ fit-codegen/
RUN cargo build --release -p app

FROM alpine:3

# Required for outbound TLS: the mailer and the OTLP exporter both rely on the system trust store,
# which the Alpine base image does not ship by default.
RUN apk add --no-cache ca-certificates

WORKDIR /activities

COPY --from=backend-builder /app/backend/target/release/app ./app
COPY --from=frontend-builder /app/frontend/build ./static

ENV SERVER_PORT=80
ENV ALLOW_ORIGIN=
ENV ACTIVITIES_STATIC_ASSETS_PATH=/activities/static

EXPOSE 80

# Probe the /health endpoint directly on the API.
HEALTHCHECK --interval=30s --timeout=3s --start-period=15s --retries=3 \
    CMD wget -q -O /dev/null http://127.0.0.1:80/health || exit 1

CMD ["/activities/app"]
