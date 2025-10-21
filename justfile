fonts := "JetBrains Mono, Fira Code, Cascadia Code, Source Code Pro, Consolas, Menlo, Monaco, DejaVu Sans Mono, monospace"
tmp-themes-dir := ".tmp/themes"
previous-tag := "git tag -l \"v*.*.*\" --merged HEAD --sort=-version:refname | head -1"

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
    gh workflow run -R pamburus/termframe release.yml --ref $(git branch --show-current) --field release-type={{type}}

[doc('Bump version')]
bump type="alpha": (setup "cargo-edit")
    cargo set-version --package termframe --bump {{type}}

[doc('Show current version')]
version: (setup "cargo" "jq")
    @cargo metadata --format-version 1 | jq -r '.packages[] | select(.name == "termframe") | .version'

[doc('List changes since the previous release')]
changes since="auto": (setup "git-cliff" "bat" "gh" "jq" "cargo")
    #!/usr/bin/env bash
    set -euo pipefail
    since=$(if [ "{{since}}" = auto ]; then {{previous-tag}}; else echo "{{since}}"; fi)
    version=$(cargo metadata --format-version 1 | jq -r '.packages[] | select(.name == "termframe") | .version')
    GITHUB_REPO=pamburus/termframe \
    GITHUB_TOKEN=$(gh auth token) \
        git-cliff --tag "v${version:?}" "${since:?}..HEAD" \
        | bat -l md --paging=never

[doc('Show previous release tag')]
previous-tag:
    @{{previous-tag}}

[doc('Run all CI checks locally')]
ci: test lint coverage

[doc('Run tests for all packages in the workspace')]
test *ARGS: (setup "cargo")
    cargo test --all-targets --all-features --workspace {{ ARGS }}

[doc('Run the Rust linter (clippy)')]
lint *ARGS: (clippy ARGS)

[doc('Run the Rust linter (clippy)')]
clippy *ARGS: (setup "clippy")
    cargo clippy --all-targets --all-features {{ ARGS }}

[doc('Update dependencies')]
update *ARGS: (setup "cargo")
    cargo update {{ ARGS }}

[doc('Update themes')]
update-themes *ARGS: (setup "cargo" "rsync")
    rm -fr "{{tmp-themes-dir}}"
    git clone -n --depth=1 --filter=tree:0 https://github.com/mbadolato/iTerm2-Color-Schemes.git "{{tmp-themes-dir}}"
    cd "{{tmp-themes-dir}}" && git sparse-checkout set --no-cone /termframe && git checkout
    rsync -a --delete "{{tmp-themes-dir}}"/termframe/ assets/themes/ --exclude-from=assets/themes/.rsync-exclude
    rm -fr "{{tmp-themes-dir}}"
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
        --mode {{mode}} \
        -o doc/help-{{mode}}.svg \
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
        --font-family "{{fonts}}" \
        --mode {{mode}} \
        --title "termframe sample" \
        --output doc/sample-{{mode}}.svg \
        ./scripts/sample {{mode}}

[doc('Generate color table screenshot')]
color-table theme mode: (setup "cargo")
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

[doc('Collect code coverage')]
coverage: (setup "coverage")
	build/ci/coverage.sh

[private]
setup *tools:
    @contrib/bin/setup.sh {{tools}}
