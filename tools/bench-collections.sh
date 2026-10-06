#!/usr/bin/env bash
# Runs the keyed-collection benchmarks (bench/collections, ROADMAP §0j step 7) on both
# backends, built for speed: `rustc -O` for Rust, `kotlinc` + `kotlin` for
# Kotlin. Prints each backend's numbers. Scratch output goes to tmp/.
#
#   tools/bench-collections.sh            both backends, three runs each
#   tools/bench-collections.sh rust 5     one backend, five runs
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
which="${1:-both}"
runs="${2:-3}"
out="$root/tmp/bench-collections"
src="$root/bench/collections/salvo"

cargo build -q --manifest-path "$root/Cargo.toml"
salvo="$root/target/debug/salvo"

if [[ "$which" == both || "$which" == rust ]]; then
    rm -rf "$out/rust"
    "$salvo" compile --backend rust --src "$src" --target "$out/rust" > /dev/null 2>&1
    rustc --edition 2021 -O "$out/rust/main.rs" -o "$out/rust/bench" 2> /dev/null
    for _ in $(seq "$runs"); do
        echo "rust:   $("$out/rust/bench" | tr '\n' ';' | sed 's/;$//; s/;/; /g')"
    done
fi

if [[ "$which" == both || "$which" == kotlin ]]; then
    rm -rf "$out/kotlin"
    "$salvo" compile --backend kotlin --src "$src" --target "$out/kotlin" > /dev/null 2>&1
    find "$out/kotlin" -name '*.kt' > "$out/kotlin-sources.txt"
    kotlinc -nowarn "@$out/kotlin-sources.txt" -d "$out/kotlin/classes" 2> /dev/null
    for _ in $(seq "$runs"); do
        echo "kotlin: $(kotlin -cp "$out/kotlin/classes" salvo.main.MainKt | tr '\n' ';' | sed 's/;$//; s/;/; /g')"
    done
fi
