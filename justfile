fonts := "JetBrains Mono, Fira Code, Cascadia Code, Source Code Pro, Consolas, Menlo, Monaco, DejaVu Sans Mono, monospace"
tmp-themes-dir := ".tmp/themes"
previous-tag := "git tag -l \"v*.*.*\" --merged HEAD --sort=-version:refname | head -1"

[private]
default:
    @just --list

# Build the project in debug mode
build *ARGS: (setup "build")
    cargo build {{ ARGS }}

# Build the project in release mode
build-release *ARGS: (setup "build")
    cargo build --locked --release {{ ARGS }}

# Run the application, example: `just run -- --help`
run *ARGS: (setup "build")
    cargo run {{ ARGS }}

# Install binary and man pages
install: && install-man-pages
    cargo install --locked --path .

# Build and publish new release
release type="patch": (setup "cargo-edit")
    gh workflow run -R pamburus/termframe release.yml --ref $(git branch --show-current) --field release-type={{type}}

# Bump version
bump type="alpha": (setup "cargo-edit")
    cargo set-version --package termframe --bump {{type}}

# List changes since the previous release
changes since="auto": (setup "git-cliff" "bat" "gh")
    #!/usr/bin/env bash
    set -euo pipefail
    since=$(if [ "{{since}}" = auto ]; then {{previous-tag}}; else echo "{{since}}"; fi)
    version=$(cargo metadata --format-version 1 | jq -r '.packages[] | select(.name == "termframe") | .version')
    GITHUB_REPO=pamburus/termframe \
    GITHUB_TOKEN=$(gh auth token) \
        git-cliff --tag "v${version:?}" "${since:?}..HEAD" \
        | bat -l md --paging=never

# Show previous release tag
previous-tag:
    @{{previous-tag}}

# Run all CI checks locally
ci: test lint coverage

# Run tests for all packages in the workspace
test *ARGS: (setup "build")
    cargo test --all-targets --all-features --workspace {{ ARGS }}

# Run the Rust linter (clippy)
lint *ARGS: (clippy ARGS)

# Run the Rust linter (clippy)
clippy *ARGS: (setup "clippy")
    cargo clippy --all-targets --all-features {{ ARGS }}

# Update dependencies
update *ARGS:
    cargo update {{ ARGS }}

# Update themes
update-themes *ARGS:
    rm -fr "{{tmp-themes-dir}}"
    git clone -n --depth=1 --filter=tree:0 https://github.com/mbadolato/iTerm2-Color-Schemes.git "{{tmp-themes-dir}}"
    cd "{{tmp-themes-dir}}" && git sparse-checkout set --no-cone /termframe && git checkout
    rsync -a --delete "{{tmp-themes-dir}}"/termframe/ assets/themes/ --exclude-from=assets/themes/.rsync-exclude
    rm -fr "{{tmp-themes-dir}}"
    cargo tidy-themes

# Tidy themes
tidy-themes *ARGS:
    cargo tidy-themes -- {{ ARGS }}

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
        -W 106 -H auto \
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
coverage: (setup "coverage")
	build/ci/coverage.sh

# Helper recipe to ensure required tools are available for a given task
[private]
setup *tools:
    @contrib/bin/setup.sh {{tools}}
