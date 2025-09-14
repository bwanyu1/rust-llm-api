#!/usr/bin/env bash
set -euo pipefail

cd /usr/src/app

# If no Cargo project is present, scaffold one (but do not run cargo).
if [ ! -f Cargo.toml ]; then
  app_name=${APP_NAME:-.}
  if [ "$app_name" = "." ]; then
    echo "No Cargo.toml found; initializing current directory as a Rust bin crate..."
    cargo init --bin --vcs none .
  else
    echo "No Cargo.toml found; creating new Rust bin crate: $app_name"
    cargo new "$app_name" --bin --vcs none
    cd "$app_name"
  fi
fi

# If a command is provided, run it; otherwise start an interactive shell.
if [ $# -gt 0 ]; then
  exec "$@"
else
  exec bash
fi
