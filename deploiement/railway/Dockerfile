# Image du relais Youma pour Railway (fiche 0030). Contexte de construction : la racine du dépôt.
# Railway fournit PORT et monte le volume (données du relais) sur /donnees ; pas de VOLUME ici (refusé par Railway).

# 1. Pages du client, du livreur et du propriétaire.
FROM node:22-bookworm-slim AS interface
WORKDIR /src/ui
COPY ui/package.json ui/package-lock.json ./
RUN npm ci
COPY ui/ ./
RUN npm run build

# 2. Le relais (un seul fichier).
FROM rust:1-bookworm AS relais
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
RUN cargo build --release --locked -p youma-relais

# 3. Image finale, petite.
FROM debian:bookworm-slim
# Pas de paquet à installer : les certificats racines (appels HTTPS vers Orange) sont intégrés au relais.
COPY --from=relais /src/target/release/youma-relais /usr/local/bin/youma-relais
COPY --from=interface /src/ui/dist /app/ui
ENV YOUMA_DONNEES=/donnees
# Railway termine le HTTPS et transmet l'adresse du client dans X-Forwarded-For (--derriere-proxy).
CMD ["sh", "-c", "mkdir -p \"$YOUMA_DONNEES\" && exec youma-relais --donnees \"$YOUMA_DONNEES\" --port \"${PORT:-8080}\" --ui /app/ui --derriere-proxy"]
