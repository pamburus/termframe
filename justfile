build *ARGS:
    cargo build {{ ARGS }}

run *ARGS:
    cargo run {{ ARGS }}

install: && install-man-pages
    cargo install --path . --locked

test *ARGS:
    cargo test --locked --all-targets --all-features {{ ARGS }}

clippy *ARGS:
    cargo clippy --locked --all-targets --all-features {{ ARGS }}

update *ARGS:
    cargo update {{ ARGS }}

install-man-pages:
    mkdir -p ~/share/man/man1
    cargo run --release --locked -- --config - --man-page >~/share/man/man1/termframe.1
    @echo $(tput bold)$(tput setaf 3)note:$(tput sgr0) ensure $(tput setaf 2)~/share/man$(tput sgr0) is added to $(tput setaf 2)MANPATH$(tput sgr0) environment variable

help:
    cargo run -- --help
