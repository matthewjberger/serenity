set windows-shell := ["powershell.exe"]

export RUST_LOG := "debug"
export RUST_BACKTRACE := "1"

@just:
    just run

check:
    cargo check --all --tests
    cargo fmt --all --check

format:
    cargo fmt --all

fix:
    cargo clippy --all --tests --fix

lint:
    cargo clippy --all --tests -- -D warnings

run:
    cargo run -r

test:
    cargo test --all -- --nocapture

@versions:
    rustc --version
    cargo fmt -- --version
    cargo clippy -- --version

