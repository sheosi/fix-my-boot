#!/bin/sh

# Note: you need cargo-appimage installed and appimagetool (with that name)
# in your PATH.

cargo build --release $@
cargo appimage $@
rm reinstall-bootloader-0.1.0-x86_64.AppImage