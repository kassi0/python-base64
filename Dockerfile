# Multi-stage build para imagem ultraleve e de alta performance em Rust
FROM rust:1-bookworm AS builder

WORKDIR /usr/src/app

# Prepara arquivos para cache de dependências
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs && cargo build --release && rm -rf src

# Copia código-fonte e assets
COPY src ./src
COPY favicon.ico .

# Compila o binário de produção
RUN touch src/main.rs && cargo build --release

# --- Imagem final enxuta de produção ---
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
 && rm -rf /var/lib/apt/lists/*

WORKDIR /opt/app

# Copia o binário estático/independente
COPY --from=builder /usr/src/app/target/release/rust-base64 /opt/app/rust-base64

# Variáveis de ambiente
ENV PORT=8080 \
    MAX_MB=10

# Ajusta permissões para execução com UID arbitrário (compatível com OpenShift e Kubernetes)
RUN chmod -R g=u /opt/app && chmod +x /opt/app/rust-base64

EXPOSE 8080

CMD ["/opt/app/rust-base64"]