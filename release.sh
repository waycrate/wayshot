#!/bin/bash
version=$(awk -F = '/version/ {print $2}' Cargo.toml | awk '{$1=$1;print}' | tr -d '"')
make
zip -r "wayshot-x86_64-$version.zip" ./target/release/wayshot
