FROM rust:1.84

# Work in a path that we will volume mount from the host
WORKDIR /usr/src/app

# Add an entrypoint that will scaffold a Rust app if none exists (no cargo run/build here)
COPY entrypoint.sh /usr/local/bin/entrypoint.sh
RUN chmod +x /usr/local/bin/entrypoint.sh

ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
# Default: keep container alive for interactive work
CMD ["bash","-lc","sleep infinity"]
