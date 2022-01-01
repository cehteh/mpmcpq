#!/bin/sh
# branch:master
# branch:devel
# branch:feature/*
cargo fmt --all -- --check && cargo check && cargo test -- --nocapture
echo $?
