_list:
    @just --list

start:
    just watch run

watch CMD='check' +ARGS='':
    watchexec \
        --watch Cargo.toml --watch src \
        --restart --clear -- just {{CMD}} {{ARGS}}

cargo +ARGS='':
    cargo {{ARGS}}

run +ARGS='':
    cargo run {{ARGS}}

check +ARGS='':
    cargo clippy {{ARGS}}

build +ARGS='':
    cargo build {{ARGS}}

test +ARGS='':
    cargo test {{ARGS}}
