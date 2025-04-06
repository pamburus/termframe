#!/bin/bash

set -e

export RUSTFLAGS="-C instrument-coverage"
export CARGO_TARGET_DIR="target/coverage"
export LLVM_PROFILE_FILE="target/coverage/test-%m-%p.profraw"
export MAIN_EXECUTABLE="target/coverage/debug/termframe"

LLVM_BIN=$(rustc --print sysroot)/lib/rustlib/$(rustc -vV | sed -n 's|host: ||p')/bin

LLVM_PROFILE_PATTERN="target/coverage/test-*.profraw"
PROFDATA_FILE="target/coverage.profdata"
IGNORE=(
    '/.cargo/git/checkouts/'
    '/.cargo/registry/'
    '/target/coverage/debug/'
    'rustc/.*/library/'
    '_capnp.rs$'
)

function executables() {
    echo ${MAIN_EXECUTABLE:?}
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
    cargo build
    ${MAIN_EXECUTABLE:?} --config - --help > /dev/null
    ${MAIN_EXECUTABLE:?} --config - --shell-completions zsh > /dev/null
    ${MAIN_EXECUTABLE:?} --config - --man-page > /dev/null
    ${MAIN_EXECUTABLE:?} --config - --list-themes > /dev/null
    ${MAIN_EXECUTABLE:?} --config - --list-themes=dark,light > /dev/null
    ${MAIN_EXECUTABLE:?} --config - --list-window-styles > /dev/null
    ${MAIN_EXECUTABLE:?} --config - --list-fonts > /dev/null
    ${MAIN_EXECUTABLE:?} --config - --mode dark > /dev/null

    local tmp=$(mktemp -t tmp.XXXXXX.svg)
    for asset in $(ls assets/test/input/*.ansi); do
        local asset_name=$(basename "${asset:?}" .ansi)
        echo "test ${asset_name:?}"
        ${MAIN_EXECUTABLE:?} --config - --mode dark <"${asset:?}" -o "${tmp:?}"
        local golden=assets/test/output/${asset_name:?}.svg
        diff "${golden:?}" "${tmp:?}"
    done
    rm -f ${tmp:?}
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
