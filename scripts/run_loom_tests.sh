#!/usr/bin/env bash

LOOM_LOG=1 \
    LOOM_LOCATION=1 \
    LOOM_CHECKPOINT_INTERVAL=1 \
    LOOM_CHECKPOINT_FILE=tmp/loom_checkpoint.json \
    RUSTFLAGS="--cfg loom --cfg loom_nightly" \
    cargo +nightly test --release --test loom