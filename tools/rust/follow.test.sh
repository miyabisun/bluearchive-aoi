#!/bin/bash
# The tests of follow.sh: its parts on sample texts, and the watching and the following in a
# scratch repository, with GitHub's answer and the checks stood in for. No network, no cargo.
#   bash tools/rust/follow.test.sh
here=$(cd "$(dirname "$0")" && pwd)
. "$here/follow.sh"

failed=0
# is <what> <got> <wanted>
is() {
  if [ "$2" != "$3" ]; then
    echo "FAILED: $1: got '$2', wanted '$3'"
    failed=1
  fi
}

# --- the list of software a run's log names (the lines as `gh run view --log` gives them)
log=$(printf '%s\n' \
  'publish	Set up job	2026-11-21T04:47:28.1Z Image: windows-2025-vs2026' \
  'publish	Set up job	2026-11-21T04:47:28.1Z Version: 20261116.315.1' \
  'publish	Set up job	2026-11-21T04:47:28.1Z Included Software: https://github.com/actions/runner-images/blob/win25-vs2026/20261116.315/images/windows/Windows2025-VS2026-Readme.md' \
  'publish	Set up job	2026-11-21T04:47:28.1Z Image Release: https://github.com/actions/runner-images/releases/tag/win25-vs2026%2F20261116.315')
is "the list a log names" "$(echo "$log" | software_list)" images/windows/Windows2025-VS2026-Readme.md
is "the list of an image on a plain branch" \
  "$(echo 'x Included Software: https://github.com/actions/runner-images/blob/main/images/ubuntu/Ubuntu2404-Readme.md' | software_list)" \
  images/ubuntu/Ubuntu2404-Readme.md
is "a log that names no list" "$(echo 'publish	Build the app	Compiling gpui' | software_list)" ""

# --- the Rust a list names: its own line, not Cargo's or Rustup's
list=$(printf '%s\n' '### Rust Tools' '- Cargo 1.100.0' '- Rust 1.100.0' '- Rustdoc 1.100.0' '- Rustup 1.29.3' '#### Packages' '- Clippy 0.1.100')
is "the Rust of a list" "$(echo "$list" | listed_rust)" 1.100.0
is "a list without Rust" "$(echo '- Ruby 3.3.10' | listed_rust)" ""

# --- the Rust pinned
toolchain=$(printf '%s\n' '[toolchain]' 'channel = "1.96.0"' 'profile = "minimal"' 'components = ["clippy", "rustfmt"]')
is "the Rust pinned" "$(echo "$toolchain" | pinned_rust)" 1.96.0
is "a channel that names no version" "$(echo 'channel = "stable"' | pinned_rust)" ""

# --- older: by the numbers, not the letters (1.96 is older than 1.100)
ordered() { if older "$1" "$2"; then echo older; else echo "not older"; fi; }
is "1.96.0 before 1.100.0" "$(ordered 1.96.0 1.100.0)" older
is "1.99.1 before 1.100.0" "$(ordered 1.99.1 1.100.0)" older
is "1.100.0 before 1.100.1" "$(ordered 1.100.0 1.100.1)" older
is "the same" "$(ordered 1.100.0 1.100.0)" "not older"
is "1.100.1 after 1.100.0" "$(ordered 1.100.1 1.100.0)" "not older"
is "2.0.0 after 1.100.0" "$(ordered 2.0.0 1.100.0)" "not older"

# --- the channel written anew: that line alone changes
is "the channel written" "$(echo "$toolchain" | pin 1.100.0)" \
  "$(printf '%s\n' '[toolchain]' 'channel = "1.100.0"' 'profile = "minimal"' 'components = ["clippy", "rustfmt"]')"

# --- watching and following, in a scratch repository
scratch=$(mktemp -d)
(
  cd "$scratch" && git init -q . && git config user.email rio@example.com && git config user.name rio &&
    git config core.autocrlf false && echo "$toolchain" > rust-toolchain.toml && git add . && git commit -q -m first
) || { echo "FAILED: no scratch repository"; exit 1; }
# run <the runners' Rust> <the checks' stand-in> [--apply]: the exit code; what was said is in $scratch.said
run() {
  (cd "$scratch" && FOLLOW_RUST_RUNNERS=$1 FOLLOW_RUST_CHECKS=$2 bash "$here/follow.sh" $3) > "$scratch.said" 2>&1
  echo $?
}
commits() { git -C "$scratch" rev-list --count HEAD; }
pinned() { pinned_rust < "$scratch/rust-toolchain.toml"; }
subject() { git -C "$scratch" log -1 --format=%s; }
changes() { git -C "$scratch" status --porcelain; }

is "behind: the watch ends with 10" "$(run 1.100.0 true)" 10
is "behind: both are told" "$(grep -c '1\.96\.0.*1\.100\.0' "$scratch.said")" 1
is "behind: the watch changes nothing" "$(pinned) $(commits) $(changes)" "1.96.0 1 "
is "nothing learned: the watch stops nobody" "$(run '' true)" 0
is "nothing learned: it says so" "$(grep -c 'could not learn' "$scratch.said")" 1
is "the runners behind this repository: nothing to do" "$(run 1.95.0 true --apply) $(pinned) $(commits)" "0 1.96.0 1"

# A check fails with the newer Rust: nothing stays changed.
is "a failing check ends the following with 1" "$(run 1.100.0 'echo the checks failed; false' --apply)" 1
is "a failing check: its words are shown" "$(grep -c 'the checks failed' "$scratch.said")" 1
is "a failing check: nothing stays changed" "$(pinned) $(commits) $(changes)" "1.96.0 1 "

# Changes of the user's in the tree: not followed over them.
echo mine > "$scratch/mine.txt"
is "a tree with changes is not followed" "$(run 1.100.0 true --apply) $(pinned) $(commits)" "2 1.96.0 1"
rm "$scratch/mine.txt"

# The checks pass (they see the newer Rust written): one commit follows.
is "following ends with 0" "$(run 1.100.0 'grep -q 1.100.0 rust-toolchain.toml' --apply)" 0
is "followed: the Rust, the commits, a clean tree" "$(pinned) $(commits) $(changes)" "1.100.0 2 "
is "followed: the commit says what it follows" "$(subject)" "chore: follow Rust 1.100.0 of GitHub's runners"
is "followed: the other lines stay" "$(grep -c 'profile = "minimal"' "$scratch/rust-toolchain.toml")" 1
is "followed: the watch is content" "$(run 1.100.0 true)" 0
is "followed: nothing more to follow" "$(run 1.100.0 false --apply) $(commits)" "0 2"

# The newer Rust asks for the code to be mended (a check fails until it is), and the mending
# builds with the newer Rust alone: --mend writes that Rust for the mending, and --apply then
# commits the mending and the Rust as one.
git -C "$scratch" reset -q --hard HEAD~1
is "back before the following" "$(pinned) $(commits)" "1.96.0 1"
is "--mend writes the newer Rust and commits nothing" "$(run 1.100.0 false --mend) $(pinned) $(commits)" "0 1.100.0 1"
is "mending: what is committed is still behind" "$(run 1.100.0 true)" 10
echo mended > "$scratch/code.txt"
is "mending: a check that still fails ends with 1" "$(run 1.100.0 false --apply)" 1
is "mending: and leaves the mending as it is" "$(pinned) $(commits) $(cat "$scratch/code.txt")" "1.100.0 1 mended"
is "mended: the following ends with 0" "$(run 1.100.0 'test -f code.txt' --apply)" 0
is "mended: the Rust, the commits, a clean tree" "$(pinned) $(commits) $(changes)" "1.100.0 2 "
is "mended: one commit holds the mending and the Rust" \
  "$(git -C "$scratch" show --name-only --format= HEAD | sort | paste -s -d ' ')" "code.txt rust-toolchain.toml"
is "mended: the commit says what it follows" "$(subject)" "chore: follow Rust 1.100.0 of GitHub's runners"

rm -rf "$scratch" "$scratch.said"
[ $failed = 0 ] && echo "follow.test.sh: ok"
exit $failed
