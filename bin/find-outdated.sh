#!/usr/bin/env bash

# Lists outdated cargo dependencies. May not be so usefulin Rust as in Scala due to Cargo, but adding just in case.
# Requires you having https://github.com/kbknapp/cargo-outdated installed
cargo outdated
