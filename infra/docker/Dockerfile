# Build stage for Svelte frontend
FROM node:24-alpine AS frontend-builder
WORKDIR /app/frontend

# Copy package.json and install dependencies
COPY client/package*.json ./
RUN npm ci

# Copy the rest of the application and build it
COPY client/svelte.config.js ./
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

# Build stage for Rust backend
FROM rust:1.89-alpine AS backend-builder

ARG APP_FEATURE

RUN apk add --no-cache musl-dev
WORKDIR /app/backend
COPY Cargo.toml Cargo.lock ./
COPY app/ app/
COPY fit-parser/ fit-parser/
COPY fit-codegen/ fit-codegen/
RUN cargo build --release -p app --features $APP_FEATURE

# Runtime stage
FROM nginx:alpine
RUN apk add --no-cache nginx

COPY --from=backend-builder /app/backend/target/release/app ./app_bin
COPY infra/docker/nginx.conf /etc/nginx/conf.d/default.conf
COPY --from=frontend-builder --chown=nginx:nginx /app/frontend/build /usr/share/nginx/html
COPY --chmod=0755 infra/docker/start.sh ./

EXPOSE 80

CMD ["./start.sh"]
