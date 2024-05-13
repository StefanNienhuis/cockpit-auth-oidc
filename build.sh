#!/bin/sh

cargo build --release --target=x86_64-unknown-linux-gnu
cargo build --release --target=aarch64-unknown-linux-gnu

cp target/x86_64-unknown-linux-gnu/release/cockpit-auth-oidc release/cockpit-auth-oidc-x86_64
cp target/aarch64-unknown-linux-gnu/release/cockpit-auth-oidc release/cockpit-auth-oidc-aarch64