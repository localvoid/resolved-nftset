#!/usr/bin/bash
set -euo pipefail

mkdir -p ${PWD}/dist/rpm/{SOURCES,SPECS,RPMS}
cp -af ${PWD}/target/release/resolved-nftset ${PWD}/resolved-nftset.service ${PWD}/dist/rpm/SOURCES/
cp ${PWD}/pkg/rpm/resolved-nftset.spec ${PWD}/dist/rpm/SPECS/

podman run --rm \
  -v ${PWD}/dist/rpm/SOURCES:/root/rpmbuild/SOURCES:Z \
  -v ${PWD}/dist/rpm/SPECS:/root/rpmbuild/SPECS:Z \
  -v ${PWD}/dist/rpm/RPMS:/root/rpmbuild/RPMS:Z \
  fedora-minimal:latest \
  /usr/bin/bash -c "
    dnf install -y rpm-build &&
    mkdir -p /root/rpmbuild/{BUILD,BUILDROOT,SRPMS} &&
    rpmbuild --define 'pkg_version ${VERSION}' -bb /root/rpmbuild/SPECS/resolved-nftset.spec
  "
