#!/bin/bash

set -euo pipefail

export RUSTFLAGS="-C instrument-coverage"
export CARGO_TARGET_DIR="target/coverage"
export LLVM_PROFILE_FILE="${PWD:?}/target/coverage/test-%m-%p.profraw"
export MAIN_EXECUTABLE="target/coverage/debug/termframe"
export TIDY_THEMES_EXE="target/coverage/debug/examples/tidy-themes"

LLVM_BIN=$(rustc --print sysroot)/lib/rustlib/$(rustc -vV | sed -n 's|host: ||p')/bin

LLVM_PROFILE_PATTERN="target/coverage/test-*.profraw"
PROFDATA_FILE="target/coverage.profdata"
IGNORE=(
    # Standard Rust directories
    '/.cargo/'
    '/.rustup/'
    # Generated code
    '_capnp\.rs$'
    # Test files themselves (we want to measure what they test, not the test code)
    '/tests\.rs$'
)

function executables() {
    echo ${MAIN_EXECUTABLE:?}
    echo ${TIDY_THEMES_EXE:?}
    cargo test --workspace --tests --no-run --message-format=json \
    | jq -r 'select(.profile.test == true) | .filenames[]' \
    | grep -v dSYM -
}

LLVM_COV_FLAGS=(
    "${IGNORE[@]/#/--ignore-filename-regex=}"
    "--instr-profile=${PROFDATA_FILE:?}"
    $(executables | xargs -I {} echo -object {})
)

function clean() {
    rm -f \
        ${LLVM_PROFILE_PATTERN:?}
}

function test() {
    cargo test --tests --workspace
    cargo build --workspace
    cargo build --manifest-path tools/tidy-themes/Cargo.toml --example tidy-themes
    ${MAIN_EXECUTABLE:?} --config - --help > /dev/null
    ${MAIN_EXECUTABLE:?} --config - --shell-completions zsh > /dev/null
    ${MAIN_EXECUTABLE:?} --config - --man-page > /dev/null
    ${MAIN_EXECUTABLE:?} --config - --list-themes > /dev/null
    ${MAIN_EXECUTABLE:?} --config - --list-themes=dark,light > /dev/null
    ${MAIN_EXECUTABLE:?} --config - --list-syntax-themes > /dev/null
    ${MAIN_EXECUTABLE:?} --config - --list-syntax-themes=dark,light > /dev/null
    ${MAIN_EXECUTABLE:?} --config - --list-window-styles > /dev/null
    ${MAIN_EXECUTABLE:?} --config - --list-fonts > /dev/null
    ${MAIN_EXECUTABLE:?} --config - --mode dark > /dev/null
    ${MAIN_EXECUTABLE:?} --config - --mode dark --show-command -- echo hello > /dev/null
    ${MAIN_EXECUTABLE:?} --config - --mode dark --show-command --syntax-theme - -- echo hello > /dev/null 2>&1
    ${MAIN_EXECUTABLE:?} --config - --mode dark --font-family "Unconfigured Font" -- echo hello > /dev/null 2>&1
    ${MAIN_EXECUTABLE:?} --config - --mode dark --font-family "Unconfigured Font" "JetBrains Mono" -- echo hello > /dev/null 2>&1

    mkdir -p target/coverage/tmp
    ${TIDY_THEMES_EXE:?} assets/themes target/coverage/tmp/.aliases.json
    diff -u assets/themes/.aliases.json target/coverage/tmp/.aliases.json
    mtime1=$(date -r assets/themes/.aliases.json +%s)

    ${TIDY_THEMES_EXE:?} assets/themes target/coverage/tmp/.aliases.json
    diff -u assets/themes/.aliases.json target/coverage/tmp/.aliases.json
    mtime2=$(date -r assets/themes/.aliases.json +%s)

    if [ ${mtime2:?} != ${mtime1:?} ]; then
        echo "Theme aliases file has been modified without modifications in the source files"
        exit 1
    fi

    ${TIDY_THEMES_EXE:?}
    git diff --exit-code -- assets/themes

    for asset in $(ls assets/test/input/*.ansi); do
        local asset_name=$(basename "${asset:?}" .ansi)
        echo "test ${asset_name:?}"
        local golden=assets/test/output/${asset_name:?}.svg
        local tmp=${golden:?}.tmp
        ${MAIN_EXECUTABLE:?} --config - --mode dark <"${asset:?}" -o "${tmp:?}"
        diff "${golden:?}" "${tmp:?}"
        rm -f ${tmp:?}
    done

    echo "test font glyphs instead of built-in glyphs"
    ${MAIN_EXECUTABLE:?} --config - --mode dark --builtin-glyphs false <assets/test/input/glyphs.ansi -o target/coverage/tmp/font-glyphs-test.svg
    rm -f target/coverage/tmp/font-glyphs-test.svg

    echo "test font subsetting"
    ${MAIN_EXECUTABLE:?} --config - --mode dark -W 80 -H 24 --embed-fonts true --subset-fonts true <assets/test/input/sample.ansi -o target/coverage/tmp/subset-test.svg
    rm -f target/coverage/tmp/subset-test.svg

    echo "test trace logging"
    TERMFRAME_DEBUG_LOG=trace ${MAIN_EXECUTABLE:?} --config - --mode dark -W 80 -H 24 --embed-fonts true --subset-fonts true <assets/test/input/sample.ansi -o /dev/null 2>/dev/null

    echo "test height with maximum only"
    ${MAIN_EXECUTABLE:?} --config - --mode dark -H ..30 <assets/test/input/sample.ansi -o /dev/null

    echo "test unknown syntax theme"
    if ${MAIN_EXECUTABLE:?} --config - --mode dark --show-command --syntax-theme no-such-theme -- echo hello >/dev/null 2>&1; then
        echo "Unknown syntax theme was accepted"
        exit 1
    fi

    echo "test missing font file"
    printf '[[fonts]]\nfamily = "Missing Font"\nfiles = ["target/coverage/tmp/missing-font.ttf"]\n' >target/coverage/tmp/missing-font.toml
    if ${MAIN_EXECUTABLE:?} --config - --config target/coverage/tmp/missing-font.toml --mode dark --font-family "Missing Font" -- echo hello >/dev/null 2>&1; then
        echo "Missing font file was accepted"
        exit 1
    fi
    rm -f target/coverage/tmp/missing-font.toml
}

function merge() {
    "${LLVM_BIN:?}/llvm-profdata" merge \
        -o ${PROFDATA_FILE:?} \
        -sparse \
        ${LLVM_PROFILE_PATTERN:?}
}

function report() {
    "${LLVM_BIN:?}/llvm-cov" \
        report \
        --show-region-summary=false \
        --show-branch-summary=false \
        --summary-only \
        "${LLVM_COV_FLAGS[@]}"
}

function publish() {
    "${LLVM_BIN:?}/llvm-cov" \
        export \
        --format="lcov" \
        "${LLVM_COV_FLAGS[@]}" \
    > target/lcov.info
}

clean; test; merge; report; publish
