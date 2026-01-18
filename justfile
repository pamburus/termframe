# Common settings

fonts := "JetBrains Mono, Fira Code, Cascadia Code, Source Code Pro, Consolas, Menlo, Monaco, DejaVu Sans Mono, monospace"
tmp-themes-dir := ".tmp/themes"
previous-tag := "git tag -l \"v*.*.*\" --merged HEAD --sort=-version:refname | head -1"

# NixOS helpers

nix-files := "."
nix-docker-image := "termframe-nixos-helper"
nix-docker-base := """
    docker run --rm \
        -t \
        --platform=linux/$(uname -m) \
        --security-opt seccomp=unconfined \
        -v "$(pwd)":/etc/nixos \
        -w /etc/nixos \
        """
nix-docker := nix-docker-base + nix-docker-image + " "

# Recipes

[private]
default:
    @just --list

[doc('Build the project in debug mode')]
build *ARGS: (setup "cargo")
    cargo build {{ ARGS }}

[doc('Build the project in release mode')]
build-release *ARGS: (setup "cargo")
    cargo build --locked --release {{ ARGS }}

[doc('Run the application, example: `just run -- --help`')]
run *ARGS: (setup "cargo")
    cargo run {{ ARGS }}

[doc('Install binary and man pages')]
install: && install-man-pages (setup "cargo")
    cargo install --locked --path .

[doc('Build and publish new release')]
release type="patch": (setup "cargo-edit")
    gh workflow run -R pamburus/termframe release.yml --ref $(git branch --show-current) --field release-type={{ type }}

[doc('Bump version')]
bump type="alpha": (setup "cargo-edit")
    cargo set-version --package termframe --bump {{ type }}

[doc('Show current version')]
version: (setup "cargo" "jq")
    @cargo metadata --format-version 1 | jq -r '.packages[] | select(.name == "termframe") | .version'

[doc('List changes since the previous release')]
changes since="auto": (setup "git-cliff" "bat" "gh" "jq" "cargo")
    #!/usr/bin/env bash
    set -euo pipefail
    since=$(if [ "{{ since }}" = auto ]; then {{ previous-tag }}; else echo "{{ since }}"; fi)
    version=$(cargo metadata --format-version 1 | jq -r '.packages[] | select(.name == "termframe") | .version')
    GITHUB_REPO=pamburus/termframe \
    GITHUB_TOKEN=$(gh auth token) \
        git-cliff --tag "v${version:?}" "${since:?}..HEAD" \
        | bat -l md --paging=never

[doc('Show previous release tag')]
previous-tag:
    @{{ previous-tag }}

[doc('Run all CI checks locally')]
ci: check-schema test lint coverage

[doc('Run tests for all packages in the workspace')]
test *ARGS: (setup "cargo")
    cargo test --all-targets --all-features --workspace {{ ARGS }}

[doc('Run the Rust linter (clippy)')]
lint *ARGS: (clippy ARGS)

[doc('Run the Rust linter (clippy)')]
clippy *ARGS: (setup "clippy")
    cargo clippy --all-targets --all-features {{ ARGS }}

[doc('Check schema validation')]
check-schema: (setup "schema")
    tombi lint
    taplo check

[doc('Format all Rust and Nix files')]
fmt: fmt-rust fmt-nix fmt-toml
    @echo "✓ All files formatted successfully"

[doc('Format Rust code')]
fmt-rust: (setup "build-nightly")
    cargo +nightly fmt --all

[doc('Format Nix files')]
fmt-nix: (run-nixfmt nix-files)

[doc('Format TOML files')]
fmt-toml: (setup "schema")
    tombi format

[doc('Check formatting without applying changes (for CI)')]
fmt-check: fmt-check-rust fmt-check-nix
    @echo "✓ Formatting is correct"

[doc('Check Rust formatting')]
fmt-check-rust: (setup "build-nightly")
    @cargo +nightly fmt --all --check

[doc('Check Nix formatting')]
fmt-check-nix:
    @if command -v nix > /dev/null; then \
        nix fmt --check; \
    fi

[doc('Update dependencies')]
update *ARGS: (setup "cargo")
    cargo update {{ ARGS }}

[doc('Update themes')]
update-themes: (setup "cargo" "rsync")
    rm -fr "{{ tmp-themes-dir }}"
    git clone -n --depth=1 --filter=tree:0 https://github.com/mbadolato/iTerm2-Color-Schemes.git "{{ tmp-themes-dir }}"
    cd "{{ tmp-themes-dir }}" && git sparse-checkout set --no-cone /termframe && git checkout
    rsync -a --delete "{{ tmp-themes-dir }}"/termframe/ assets/themes/ --exclude-from=assets/themes/.rsync-exclude
    rm -fr "{{ tmp-themes-dir }}"
    cargo tidy-themes

[doc('Tidy themes')]
tidy-themes *ARGS: (setup "cargo")
    cargo tidy-themes -- {{ ARGS }}

[doc('Install man pages')]
install-man-pages: (setup "cargo")
    mkdir -p ~/share/man/man1
    cargo run --release --locked -- --config - --man-page >~/share/man/man1/termframe.1
    @echo $(tput bold)$(tput setaf 3)note:$(tput sgr0) ensure $(tput setaf 2)~/share/man$(tput sgr0) is added to $(tput setaf 2)MANPATH$(tput sgr0) environment variable

[doc('Generate help page screenshots')]
help: (help-for "dark") (help-for "light")

[private]
help-for mode: (build "--locked")
    target/debug/termframe \
        --title 'termframe --help' \
        --mode {{ mode }} \
        -o doc/help-{{ mode }}.svg \
        -W 106 -H auto \
        -- ./target/debug/termframe --config - --help

[doc('Generate sample screenshots')]
sample: (sample-for "dark") (sample-for "light")

[private]
sample-for mode: (setup "cargo")
    cargo run --locked -- \
        --config - \
        -W 79 -H 24 \
        --embed-fonts true \
        --font-family "{{ fonts }}" \
        --mode {{ mode }} \
        --title "termframe sample" \
        --output doc/sample-{{ mode }}.svg \
        ./scripts/sample {{ mode }}

[doc('Generate color table screenshot')]
color-table theme mode: (setup "cargo")
    cargo run --locked -- \
        --config - \
        -W 80 -H 40 \
        --embed-fonts true \
        --font-family "{{ fonts }}" \
        --mode {{ kebabcase(mode) }} \
        --theme "{{ kebabcase(theme) }}" \
        --bold-is-bright true \
        --bold-font-weight normal \
        --title "{{ theme }} ({{ mode }})" \
        --output doc/color-table-{{ kebabcase(theme) }}-{{ kebabcase(mode) }}.svg \
        ./scripts/color-table.sh

[doc('Collect code coverage')]
coverage: (setup "coverage")
    build/ci/coverage.sh

[doc('Show uncovered changed lines comparing to {{base}}')]
uncovered base="origin/main": (setup "coverage")
    @scripts/coverage-diff-analysis.py -q --ide-links {{ base }}

[doc('Update Nix flakes')]
update-nix: (run-nix "flake" "update")

# Helper function to run a command locally or in Docker if not installed
[private]
nix-run-local-or-docker docker cmd *args:
    #!/usr/bin/env bash
    set -euo pipefail
    if command -v {{ cmd }} >/dev/null 2>&1; then
        {{ cmd }} {{ args }}
    else
        just build-nix-docker-image
        {{ docker }} {{ cmd }} {{ args }}
    fi

# Helper function to run nix commands locally or in Docker
[private]
run-nix *args: (nix-run-local-or-docker nix-docker "nix" args)

# Helper function to run nixfmt commands locally or in Docker
[private]
run-nixfmt *args: (nix-run-local-or-docker nix-docker "nixfmt" args)

# Helper function to build NixOS docker image
[private]
build-nix-docker-image:
    docker build -t {{ nix-docker-image }} build/docker/nix

[private]
setup *tools:
    @contrib/bin/setup.sh {{ tools }}
