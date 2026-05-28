#!/usr/bin/bash
set -euo pipefail

mkdir -p ${PWD}/dist/deb/DEBIAN
mkdir -p ${PWD}/dist/deb/usr/bin
mkdir -p ${PWD}/dist/deb/usr/lib/systemd/system

cp ${PWD}/pkg/deb/control ${PWD}/dist/deb/DEBIAN/
install -m 755 ${PWD}/pkg/deb/prerm ${PWD}/dist/deb/DEBIAN/
cp -af ${PWD}/target/release/resolved-nftset ${PWD}/dist/deb/usr/bin/
cp -af ${PWD}/resolved-nftset.service ${PWD}/dist/deb/usr/lib/systemd/system/

sed -i "s/__VERSION__/${VERSION}/g" ${PWD}/dist/deb/DEBIAN/control
sed -i "s/__ARCH__/${ARCH}/g" ${PWD}/dist/deb/DEBIAN/control

podman run --rm \
  -v ${PWD}/dist/deb:/pkg/deb:Z \
  -v ${PWD}/dist:/pkg/out:Z \
  debian:latest \
  /bin/bash -c "
    apt-get update && apt-get install -y dpkg-dev &&
    dpkg-deb --build --root-owner-group /pkg/deb /pkg/out/resolved-nftset_${VERSION}_${ARCH}.deb
  "
