#!/usr/bin/env bash

set -euo pipefail

BINARY=resolved-nftset
INSTALL_BIN=/usr/local/bin
INSTALL_CFG=/etc/resolved-nftset
INSTALL_NFT=/etc/nftables.d
UNIT_DIR=/etc/systemd/system

need_root() { [[ $EUID -eq 0 ]] || { echo "Run as root (or sudo)."; exit 1; }; }

build() {
    echo "==> Building release binary…"
    cargo build --release
    echo "    Built: target/release/$BINARY"
}

install_files() {
    need_root
    echo "==> Installing binary → $INSTALL_BIN/$BINARY"
    install -m 755 "target/release/$BINARY" "$INSTALL_BIN/$BINARY"

    echo "==> Creating config dir → $INSTALL_CFG"
    mkdir -p "$INSTALL_CFG"

    echo "==> Installing systemd unit → $UNIT_DIR/resolved-nftset.service"
    install -m 644 resolved-nftset.service "$UNIT_DIR/resolved-nftset.service"
    systemctl daemon-reload

    echo ""
    echo "==> Done.  Next steps:"
    echo "    1. Edit $INSTALL_CFG/table_name/set_name/domains"
    echo "    2. Enable service:   systemctl enable --now resolved-nftset"
}

case "${1:-all}" in
    build)  build ;;
    install) install_files ;;
    all)    build; install_files ;;
    *)      echo "Usage: $0 [build|install|all]"; exit 1 ;;
esac
