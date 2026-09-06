# ⚠️ INCOMPLETO — NÃO CONSTRÓI AINDA (parado em 13/08/2026 a pedido do titular).
# Falta `docker/validar-linux.sh`, o roteiro que o ENTRYPOINT chama: copiar
# /repo (montado somente leitura) para /work, e então rodar npm install, build,
# vitest, ícones, download dos binários embutidos, geração dos fixtures de PDF,
# cargo check, cargo test --lib e o smoke de exportação. Sem esse arquivo o
# `docker build` falha no COPY. Ver docs/_interno/STATUS_HANDOFF.md.
#
# Ambiente de validação em Linux — espelha o runner do CI (ubuntu-22.04).
#
# Serve para rodar a suíte inteira fora do Windows antes de empurrar: suíte
# local verde num só sistema operacional não é a suíte verde.
#
# A imagem carrega só a cadeia de ferramentas; o código entra por montagem, em
# tempo de execução. Assim editar um arquivo não obriga a reconstruir a imagem,
# e a cópia de trabalho fica dentro do contêiner — o `node_modules` e o
# `target/` do Linux nunca encostam nos do hospedeiro.
#
# Construir:
#   docker build -t markforge-linux -f docker/Dockerfile docker
#
# Rodar (a partir da raiz do repositório):
#   docker run --rm -v "$PWD":/repo:ro markforge-linux
#
# Rodar só uma parte:
#   docker run --rm -v "$PWD":/repo:ro markforge-linux --somente-rust

FROM ubuntu:22.04

ENV DEBIAN_FRONTEND=noninteractive \
    CARGO_HOME=/opt/cargo \
    RUSTUP_HOME=/opt/rustup \
    PATH=/opt/cargo/bin:$PATH \
    CARGO_TERM_COLOR=always

# Dependências de sistema: as mesmas do job de CI, mais o que a cópia de
# trabalho e o download dos binários embutidos precisam.
RUN apt-get update && apt-get install -y --no-install-recommends \
        libwebkit2gtk-4.1-dev \
        libappindicator3-dev \
        librsvg2-dev \
        patchelf \
        jq \
        curl \
        ca-certificates \
        xz-utils \
        git \
        rsync \
        build-essential \
        pkg-config \
        libssl-dev \
        python3 \
        python3-pip \
    && rm -rf /var/lib/apt/lists/*

# reportlab e pillow geram os fixtures de PDF — que não vivem no repositório.
RUN python3 -m pip install --no-cache-dir --disable-pip-version-check reportlab pillow

# Node 20, igual ao CI.
RUN curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
    && apt-get install -y --no-install-recommends nodejs \
    && rm -rf /var/lib/apt/lists/*

# Rust estável.
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
        | sh -s -- -y --profile minimal --default-toolchain stable

COPY validar-linux.sh /usr/local/bin/validar-linux
RUN chmod +x /usr/local/bin/validar-linux

WORKDIR /work
ENTRYPOINT ["/usr/local/bin/validar-linux"]
