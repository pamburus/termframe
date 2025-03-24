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

sample: (sample-for-mode "dark") (sample-for-mode "light")

sample-for-mode mode:
    cargo run --locked -- \
        --config - \
        -W 79 -H 24 \
        --embed-fonts true \
        --font-family "JetBrains Mono, Fira Code, Cascadia Code, Source Code Pro, Consolas, Menlo, Monaco, DejaVu Sans Mono, monospace" \
        --mode {{mode}} \
        --title "termframe sample" \
        --output doc/sample-{{mode}}.svg \
        ./scripts/sample.sh {{mode}}
