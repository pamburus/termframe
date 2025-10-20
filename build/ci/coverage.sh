#!/bin/bash

set -euo pipefail

export RUSTFLAGS="-C instrument-coverage"
export CARGO_TARGET_DIR="target/coverage"
export LLVM_PROFILE_FILE="target/coverage/test-%m-%p.profraw"
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
    ${MAIN_EXECUTABLE:?} --config - --list-window-styles > /dev/null
    ${MAIN_EXECUTABLE:?} --config - --list-fonts > /dev/null
    ${MAIN_EXECUTABLE:?} --config - --mode dark > /dev/null

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

    for asset in $(ls assets/test/input/*.ansi); do
        local asset_name=$(basename "${asset:?}" .ansi)
        echo "test ${asset_name:?}"
        local golden=assets/test/output/${asset_name:?}.svg
        local tmp=${golden:?}.tmp
        ${MAIN_EXECUTABLE:?} --config - --mode dark -W 80 -H 24 <"${asset:?}" -o "${tmp:?}"
        diff "${golden:?}" "${tmp:?}"
        rm -f ${tmp:?}
    done
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
