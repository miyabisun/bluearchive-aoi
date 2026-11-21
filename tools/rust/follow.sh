#!/bin/bash
# Keeps the Rust this repository builds with (the channel of rust-toolchain.toml, written
# nowhere else) at the one GitHub's runners carry: the newest stable GitHub has taken up,
# and, being the runners' own, the one Rust the release's cache keys on.
#   bash tools/rust/follow.sh            tells both; ends with 10 when this repository is behind
#   bash tools/rust/follow.sh --apply    follows: writes the newer Rust, runs every check with
#                                        it and commits; leaves nothing changed if a check fails
#   bash tools/rust/follow.sh --mend     for a newer Rust whose checks ask for the code to be
#                                        mended: writes that Rust and commits nothing; with the
#                                        code mended, --apply commits the mending and the Rust
#                                        as one (the mending may not build with the older Rust)
# The gate (.githooks/pre-push) follows on every push of main; FOLLOW_RUST=off passes it by.
# What GitHub's runners carry is read from the list of software of their image: the list the
# last release's run named, as GitHub has it now. Without gh or a connection nothing is
# known, which is told and stops nobody.

# The list of the software of the image the releases run on, where no run's log names one.
SOFTWARE_LIST=images/windows/Windows2025-VS2026-Readme.md

# The path of an image's list of software in GitHub's runner-images, as a run's log names it
# ("Included Software: https://github.com/actions/runner-images/blob/<ref>/<path>"); stdin.
software_list() {
  # The leading ".*" is greedy: the last "images/" of the line starts the path.
  sed -n 's,.*Included Software: .*/\(images/[^[:space:]]*\.md\).*,\1,p' | head -n 1
}

# The Rust such a list names (its line "- Rust 1.100.0"); stdin.
listed_rust() {
  sed -n 's/^- Rust \([0-9][0-9.]*\)[[:space:]]*$/\1/p' | head -n 1
}

# The Rust pinned: the version rust-toolchain.toml's channel names; stdin. Nothing for a
# channel that names no version ("stable").
pinned_rust() {
  sed -n 's/^channel *= *"\([0-9][0-9.]*\)".*/\1/p' | head -n 1
}

# Whether the version $1 is older than the version $2, number by number.
older() {
  local ours theirs at
  IFS=. read -r -a ours <<< "$1"
  IFS=. read -r -a theirs <<< "$2"
  for at in 0 1 2; do
    # 10#: a number with a leading zero is no octal one.
    if ((10#${ours[at]:-0} < 10#${theirs[at]:-0})); then return 0; fi
    if ((10#${ours[at]:-0} > 10#${theirs[at]:-0})); then return 1; fi
  done
  return 1
}

# rust-toolchain.toml with its channel set to $1; stdin to stdout.
pin() {
  sed "s/^channel *= *\"[^\"]*\"/channel = \"$1\"/"
}

# The Rust GitHub's runners carry now; nothing where it cannot be learned.
runners_rust() {
  command -v gh > /dev/null 2>&1 || return 0
  local run list
  # The image the releases run on is told by the last release's log, with its list of software.
  run=$(gh run list --workflow Release --status success --limit 1 --json databaseId --jq '.[0].databaseId' 2> /dev/null)
  if [ -n "$run" ]; then
    list=$(gh run view "$run" --log 2> /dev/null | software_list)
  fi
  # The list as GitHub has it now, not as it was when that release ran.
  gh api -H "Accept: application/vnd.github.raw+json" \
    "repos/actions/runner-images/contents/${list:-$SOFTWARE_LIST}" 2> /dev/null | listed_rust
}

# Writes the Rust $1 into the working tree's rust-toolchain.toml.
write_rust() {
  pin "$1" < rust-toolchain.toml > rust-toolchain.toml.new && mv rust-toolchain.toml.new rust-toolchain.toml
}

# Follows the Rust $1: writes it in place of the pinned one, runs every check with it and
# commits; a check that fails puts the pinned one back. Where the working tree has that Rust
# written already ($2: --mend, and the code mended for it), the checks run on the tree as it
# is and the commit takes all of it; a check that fails leaves the mending where it is.
# The checks install the Rust first (rustup reads rust-toolchain.toml); FOLLOW_RUST_CHECKS
# stands in for them in the tests.
follow() {
  local theirs=$1 written=$2 mending=
  if [ "$written" = "$theirs" ]; then
    mending=yes
  elif [ -n "$(git status --porcelain)" ]; then
    echo "follow: the working tree has changes: commit or stash them, then follow" >&2
    return 2
  else
    write_rust "$theirs" || return 2
  fi
  if ! sh -c "${FOLLOW_RUST_CHECKS:-rustup toolchain install && sh tools/checks.sh}"; then
    if [ -n "$mending" ]; then
      echo "follow: a check still fails with Rust $theirs: nothing was committed, the mending stays as it is" >&2
    else
      git checkout -q -- rust-toolchain.toml
      echo "follow: a check failed with Rust $theirs: nothing was changed. To mend the code for it:" >&2
      echo "  bash tools/rust/follow.sh --mend    (writes Rust $theirs; then mend, and --apply commits both)" >&2
    fi
    return 1
  fi
  if [ -n "$mending" ]; then git add -A; else git add rust-toolchain.toml; fi &&
    git commit -q -m "chore: follow Rust $theirs of GitHub's runners" &&
    echo "followed: $(git log -1 --format='%h %s')"
}

main() {
  cd "$(git rev-parse --show-toplevel)" || exit 2
  local ours written theirs
  # The Rust committed is the one a push sends; the working tree's is the newer one already
  # while the code is being mended for it.
  ours=$(git show HEAD:rust-toolchain.toml 2> /dev/null | pinned_rust)
  written=$(pinned_rust < rust-toolchain.toml)
  if [ -z "$ours" ]; then
    echo "follow: rust-toolchain.toml pins no version: nothing to follow" >&2
    exit 0
  fi
  # FOLLOW_RUST_RUNNERS stands in for GitHub's answer in the tests.
  if [ -n "${FOLLOW_RUST_RUNNERS+set}" ]; then
    theirs=$FOLLOW_RUST_RUNNERS
  else
    theirs=$(runners_rust)
  fi
  if [ -z "$theirs" ]; then
    echo "follow: could not learn the Rust of GitHub's runners (no gh, or no connection): Rust $ours is not checked" >&2
    exit 0
  fi
  if ! older "$ours" "$theirs"; then
    echo "Rust $ours: not behind GitHub's runners, which carry $theirs"
    exit 0
  fi
  echo "Rust $ours is behind GitHub's runners, which carry $theirs"
  case "${1-}" in
    --apply)
      follow "$theirs" "$written"
      exit $?
      ;;
    --mend)
      write_rust "$theirs" || exit 2
      echo "Rust $theirs is written, not committed: mend what the checks tell (sh tools/checks.sh),"
      echo "then bash tools/rust/follow.sh --apply commits the mending with it."
      echo "To give it up: git checkout -- rust-toolchain.toml"
      exit 0
      ;;
    *)
      echo "follow it: bash tools/rust/follow.sh --apply"
      exit 10
      ;;
  esac
}

# Sourced, only the parts are taken (the tests).
if [ "${BASH_SOURCE[0]}" = "$0" ]; then
  main "$@"
fi
