#!/bin/sh
set -eu

if [ "$(uname -s)" = Darwin ]; then
    SDKROOT="$(xcrun --show-sdk-path)"
    CC=/usr/bin/clang
    CFLAGS="-isysroot $SDKROOT"
    export SDKROOT CC CFLAGS
fi
mkdir -p target
cargo run --locked --quiet -p sunstack-lint --example export_schema > target/sunstack.lint.schema.json
