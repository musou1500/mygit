#!/bin/sh

cargo run --bin main init
tree_hash=$(cargo run --bin main write-tree)
commit_hash=$(cargo run --bin main commit-tree $tree_hash -m "Initial commit from mygit")

mkdir -p .git/refs/heads
echo $commit_hash > .git/refs/heads/main
