# Build Stage
FROM rust:nightly as builder

WORKDIR /usr/src/app
COPY . .

# Install dependencies for build (e.g. OpenSSL)
RUN apt-get update && apt-get install -y pkg-config libssl-dev git && rm -rf /var/lib/apt/lists/*

RUN cargo build --release

# Runtime Stage
FROM debian:bookworm-slim

# Install runtime dependencies
# - git: for cloning the repo
# - ca-certificates: to trust HTTPS
# - openssl: for crypto
# - kubeseal: we need to fetch this binary
RUN apt-get update && apt-get install -y \
    git \
    ca-certificates \
    openssl \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Install kubeseal
ARG KUBESEAL_VERSION=0.24.4
RUN curl -L https://github.com/bitnami-labs/sealed-secrets/releases/download/v${KUBESEAL_VERSION}/kubeseal-${KUBESEAL_VERSION}-linux-amd64.tar.gz -o kubeseal.tar.gz \
    && tar -xzf kubeseal.tar.gz kubeseal \
    && mv kubeseal /usr/local/bin/ \
    && rm kubeseal.tar.gz

# Install kubectl (for fallback retrieval)
RUN curl -LO "https://dl.k8s.io/release/$(curl -L -s https://dl.k8s.io/release/stable.txt)/bin/linux/amd64/kubectl" \
    && install -o root -g root -m 0755 kubectl /usr/local/bin/kubectl

WORKDIR /app
COPY --from=builder /usr/src/app/target/release/rotato /usr/local/bin/rotato

# Entrypoint script to handle git cloning? 
# Or just run the binary and expect repo to be mounted or cloned by init container?
# The plan said CronJob clones the repo. Let's make the entrypoint flexible.
# We'll use a script to clone and run.

COPY docker-entrypoint.sh /usr/local/bin/
RUN chmod +x /usr/local/bin/docker-entrypoint.sh

ENTRYPOINT ["docker-entrypoint.sh"]
