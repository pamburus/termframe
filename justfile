fonts := "JetBrains Mono, Fira Code, Cascadia Code, Source Code Pro, Consolas, Menlo, Monaco, DejaVu Sans Mono, monospace"
tmp-themes-dir := ".tmp/themes"

[private]
default:
    @just --list

# Build the project in debug mode
build *ARGS:
    cargo build {{ ARGS }}

# Build the project in release mode
build-release *ARGS:
    cargo build --locked --release {{ ARGS }}

# Run the application, example: `just run -- --help`
run *ARGS:
    cargo run {{ ARGS }}

# Install binary and man pages
install: && install-man-pages
    cargo install --locked --path .

# Run all CI checks locally
ci: test lint

# Run tests for all packages in the workspace
test *ARGS:
    cargo test --all-targets --all-features --workspace {{ ARGS }}

# Run the Rust linter (clippy)
lint *ARGS: (clippy ARGS)

# Run the Rust linter (clippy)
clippy *ARGS:
    cargo clippy --all-targets --all-features {{ ARGS }}

# Update dependencies
update *ARGS:
    cargo update {{ ARGS }}

# Update themes
update-themes *ARGS:
    rm -fr "{{tmp-themes-dir}}"
    git clone -n --depth=1 --filter=tree:0 git@github.com:mbadolato/iTerm2-Color-Schemes.git "{{tmp-themes-dir}}"
    cd "{{tmp-themes-dir}}" && git sparse-checkout set --no-cone /termframe && git checkout
    mv "{{tmp-themes-dir}}"/termframe/* assets/themes/
    rm -fr "{{tmp-themes-dir}}"
    cargo build

# Install man pages
install-man-pages:
    mkdir -p ~/share/man/man1
    cargo run --release --locked -- --config - --man-page >~/share/man/man1/termframe.1
    @echo $(tput bold)$(tput setaf 3)note:$(tput sgr0) ensure $(tput setaf 2)~/share/man$(tput sgr0) is added to $(tput setaf 2)MANPATH$(tput sgr0) environment variable

# Generate help page screenshots
help: (help-for "dark") (help-for "light")

[private]
help-for mode: (build "--locked")
    target/debug/termframe \
        --title 'termframe --help' \
        --mode {{mode}} \
        -o doc/help-{{mode}}.svg \
        -W 104 -H 51 \
        -- ./target/debug/termframe --config - --help

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
        ./scripts/sample {{mode}}

# Generate color table screenshot
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

# Collect code coverage
coverage: contrib-coverage
	build/ci/coverage.sh

[private]
contrib-coverage:
	contrib/bin/setup.sh coverage
