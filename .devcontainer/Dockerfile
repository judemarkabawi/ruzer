# FROM mcr.microsoft.com/devcontainers/base:ubuntu

# RUN apt-get update && apt-get install -y \
#     cmake \
#     libexpat1-dev \
#     libfontconfig-dev \
#     libfreetype-dev \
#     libxkbcommon-dev \
#     pkgconf \
#     libwayland-dev \
#     mesa-utils \
#     libglu1-mesa-dev \
#     && apt-get clean \
#     && rm -rf /var/lib/apt/lists/*

FROM docker.io/fedora

RUN dnf install -y \
    gtk4-devel libadwaita-devel libGLES gcc \
    rust cargo clippy rust-src rustfmt
