#!/bin/bash
set -e

# Install required SDK extension
flatpak install --user --noninteractive org.gnome.Sdk//50 org.gnome.Platform//50 org.freedesktop.Sdk.Extension.rust-stable//25.08

# Define the manifest to use
MANIFEST="packaging/io.github.tobagin.Sidestep.yml"

if [[ "$1" == "--dev" ]]; then
    MANIFEST="packaging/io.github.tobagin.Sidestep.Dev.yml"
    echo "Building Development version..."
else
    echo "Building Production version..."
fi

# Build the flatpak
flatpak-builder --user --install --force-clean build-dir "$MANIFEST"
