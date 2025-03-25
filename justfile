fonts := "JetBrains Mono, Fira Code, Cascadia Code, Source Code Pro, Consolas, Menlo, Monaco, DejaVu Sans Mono, monospace"

[doc('build')]
build *ARGS:
    cargo build {{ ARGS }}

[doc('run')]
run *ARGS:
    cargo run {{ ARGS }}

[doc('install')]
install: && install-man-pages
    cargo install --path . --locked

[doc('run tests')]
test *ARGS:
    cargo test --locked --all-targets --all-features {{ ARGS }}

[doc('run linters')]
clippy *ARGS:
    cargo clippy --locked --all-targets --all-features {{ ARGS }}

[doc('update dependencies')]
update *ARGS:
    cargo update {{ ARGS }}

[doc('install man pages')]
install-man-pages:
    mkdir -p ~/share/man/man1
    cargo run --release --locked -- --config - --man-page >~/share/man/man1/termframe.1
    @echo $(tput bold)$(tput setaf 3)note:$(tput sgr0) ensure $(tput setaf 2)~/share/man$(tput sgr0) is added to $(tput setaf 2)MANPATH$(tput sgr0) environment variable

[doc('generate help page')]
help:
    cargo run -- --help

[doc('generate sample screenshots')]
sample: (sample-for "dark") (sample-for "light")

[private]
sample-for mode:
    cargo run --locked -- \
        --config - \
        -W 79 -H 24 \
        --embed-fonts true \
        --font-family "{{fonts}}" \
        --mode {{mode}} \
        --title "termframe sample" \
        --output doc/sample-{{mode}}.svg \
        ./scripts/sample.sh {{mode}}

[doc('generate color table screenshot')]
color-table theme mode:
    cargo run --locked -- \
        --config - \
        -W 80 -H 40 \
        --embed-fonts true \
        --font-family "{{fonts}}" \
        --mode {{kebabcase(mode)}} \
        --theme "{{kebabcase(theme)}}" \
        --bold-is-bright true \
        --bold-font-weight normal \
        --title "{{theme}} ({{mode}})" \
        --output doc/color-table-{{kebabcase(theme)}}-{{kebabcase(mode)}}.svg \
        ./scripts/color-table.sh
