# Contributing to termframe

## Development setup

The Rust toolchain version is pinned in `rust-toolchain.toml`. Common tasks are available as [just](https://github.com/casey/just) recipes; run `just` to list them all.

| Task | Command |
| --- | --- |
| Build | `just build` |
| Run | `just run -- <args>` |
| Run tests | `just test` |
| Run the linter | `just lint` |
| Format code | `just fmt` |
| Show changed lines not covered by tests | `just uncovered` |
| Bump the version | `just bump` |
| Run all CI checks | `just ci` |

## Workflow

1. **Issue first.** Each problem or feature gets its own GitHub issue before work starts. If you discover an unrelated problem along the way, file a separate issue for it rather than fixing it in the same change.
2. **One change per pull request.** Unrelated fixes go into separate pull requests, each based on the latest `main`.
3. **One commit per pull request.** Amend the commit and force-push (`git push --force-with-lease`) instead of adding follow-up commits. After rebasing onto a newer `main`, check that the version bump is still part of the diff.
4. **Version bump.** Every pull request runs `just bump` once, which increments the alpha pre-release version in `Cargo.toml` and `Cargo.lock`.
5. **Link the issue.** The pull request body says `Closes #<issue>`; the commit message uses a `Refs: #<issue>` footer.

## Tests

* Unit tests live in a separate `tests.rs` file in the directory of the module they test: `src/foo.rs` declares `#[cfg(test)] mod tests;`, and the tests go into `src/foo/tests.rs`. Do not write inline `mod tests { ... }` blocks.
* Files named `tests.rs` are excluded from coverage, so test code never affects coverage numbers.
* `src/main.rs` has no module directory of its own. Put logic that needs testing into a library module and test it there.
* Golden tests render the inputs in `assets/test/input/*.ansi` and compare the result with `assets/test/output/*.svg`. They run as part of `just coverage`.

## Coverage

Every changed line must be covered by tests. Before opening a pull request, run `just uncovered`, which measures coverage and lists changed lines that are not covered. It must report nothing.

## Evidence

Issues and pull requests include real output, not just a description of the expected result. For behavior that is visible in logs, run with debug logging enabled and include the relevant lines, before and after the change:

```sh
TERMFRAME_DEBUG_LOG=debug termframe ...
```

If some evidence cannot be collected, for example for a platform you have no access to, say so explicitly.

## Commit messages and pull requests

Commit messages follow [Conventional Commits](https://www.conventionalcommits.org/), for example `fix(fonts): load fonts from local files`. They are used to generate the changelog, so write them for users:

* Use the imperative mood and keep the title under 70 characters.
* Add a scope for the affected component when there is one.
* Describe what changes for the user, not how the code changed. Implementation details belong in code comments.
* For changes with no user-visible effect, such as CI, build or dependency updates, describe the technical change plainly.

Pull request titles follow the same format. The body briefly explains the change, links the issue and includes a verification checklist with the evidence described above.
