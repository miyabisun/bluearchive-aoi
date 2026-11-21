#!/bin/sh
# Every check a commit has to pass before it reaches GitHub, in one place: the gate
# (.githooks/pre-push) runs them, and so does the following of GitHub's Rust
# (tools/rust/follow.sh --apply) before it commits the newer one. Nothing else tests them.
#   sh tools/checks.sh        from the repository's root
bash tools/rust/follow.test.sh || exit 1
cargo fmt --check || exit 1
cargo clippy --all-targets --locked -- -D warnings || exit 1
cargo test --locked || exit 1
