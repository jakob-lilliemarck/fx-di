#!/usr/bin/env bash
set -euo pipefail

err() {
    printf 'error: %s\n' "$*" >&2
    exit 1
}

# --- Prerequisites ---
command -v cargo >/dev/null 2>&1 || err "cargo is not installed or not on PATH"
command -v git >/dev/null 2>&1 || err "git is not installed or not on PATH"

# --- Token ---
TOKEN="${CRATES_IO_TOKEN:-}"
[ -n "$TOKEN" ] || err "CRATES_IO_TOKEN is not set (export CRATES_IO_TOKEN=...)"

# --- Clean working tree ---
if [ -n "$(git status --porcelain)" ]; then
    echo "error: working directory is not clean. Commit or stash changes first." >&2
    git status --short
    exit 1
fi

# --- Quality gates ---
echo "Checking formatting..."
cargo fmt --check || err "code is not formatted — run 'cargo fmt', review the diff, and commit it"

echo "Linting..."
cargo clippy --all-targets -- -D warnings || err "cargo clippy failed"

echo "Running tests..."
cargo test || err "cargo test failed"

echo "Building docs..."
cargo doc --no-deps || err "cargo doc failed"

# --- Version bump ---
current_version=$(cargo pkgid | cut -d'#' -f2) || err "could not determine current version"
echo "Current version: $current_version"
read -rp "Bump major, minor, or patch? " part

IFS='.' read -r major minor patch <<< "$current_version"
case "$part" in
    major) major=$((major + 1)); minor=0; patch=0 ;;
    minor) minor=$((minor + 1)); patch=0 ;;
    patch) patch=$((patch + 1)) ;;
    *) err "invalid choice '$part' (expected major, minor, or patch)" ;;
esac
new_version="$major.$minor.$patch"

echo "Bumping: $current_version -> $new_version"
sed -i "s/^version = \".*\"/version = \"$new_version\"/" Cargo.toml

# --- Sync Cargo.lock (root version only; does not touch deps) ---
cargo check || {
    echo "error: failed to sync Cargo.lock. Revert with: git checkout Cargo.toml Cargo.lock" >&2
    exit 1
}

# --- Validate the new version + packaging before committing anything ---
cargo publish --dry-run --allow-dirty || {
    echo "error: cargo publish --dry-run failed. Revert with: git checkout Cargo.toml Cargo.lock" >&2
    exit 1
}

# --- Validate token ---
echo "$TOKEN" | cargo login || err "cargo login failed (is CRATES_IO_TOKEN valid?)"

# --- Human confirmation ---
echo "Ready to release v$new_version."
read -rp "Commit, tag, publish to crates.io, and push? [y/N] " confirm
case "$confirm" in
    [Yy] | [Yy][Ee][Ss]) ;;
    *) echo "Aborted. Revert with: git checkout Cargo.toml Cargo.lock"; exit 0 ;;
esac

# --- Release ---
git add Cargo.toml Cargo.lock
git commit -m "chore: bump version to $new_version"
git tag "v$new_version" -m "Release v$new_version"

cargo publish

git push origin HEAD
git push origin "v$new_version"

echo "Published v$new_version."
