#!/usr/bin/env bash
# VENN_CLEANUP_MEASUREMENT: temporary paired-ref runner; VC-FINAL removes this file.
set -euo pipefail

baseline=${1:?baseline main SHA required}
head_ref=${2:?exact PR head SHA required}
[[ $baseline =~ ^[0-9a-f]{40}$ && $head_ref =~ ^[0-9a-f]{40}$ ]]
repository=$(git rev-parse --show-toplevel)
workspace=${GITHUB_WORKSPACE:-$repository}
scratch=$(mktemp -d "$workspace/venn-cleanup-measurement.XXXXXX")
cleanup() {
    for label in baseline head; do
        if [[ -d $scratch/$label ]]; then
            git -C "$repository" worktree remove --force "$scratch/$label"
        fi
    done
    rm -rf -- "$scratch"
}
trap cleanup EXIT

git fetch --no-tags origin "$baseline" "$head_ref"
printf 'VENN_CLEANUP_MEASUREMENT baseline=%s head=%s\n' "$baseline" "$head_ref"
printf 'runner=%s os=%s arch=%s image=%s version=%s run=%s attempt=%s\n' \
    "${RUNNER_NAME:-local}" "${RUNNER_OS:-unknown}" "${RUNNER_ARCH:-unknown}" \
    "${ImageOS:-unknown}" "${ImageVersion:-unknown}" "${GITHUB_RUN_ID:-local}" "${GITHUB_RUN_ATTEMPT:-1}"
uname -a
rustc -Vv
cargo -V
printf 'feature=ncolors_6 profile=release RUSTFLAGS=%s output=quiet counter expected=233\n' "${RUSTFLAGS:-}"
printf 'protocol=both builds first; one warmup/ref; five samples/ref; alternating order; serial\n'

# The same cfg(test)-only layout probe can inspect private entries in either ref.
# Its exact injection/diff/hash is recorded; baseline production code is unchanged.
cat > "$scratch/layout.rs" <<'RUST'

#[cfg(test)]
mod venn_cleanup_measurement_layout {
    #[test]
    #[ignore = "VENN_CLEANUP_MEASUREMENT"]
    fn entry_layout() {
        let trail = super::Trail::new();
        println!("VENN_CLEANUP_MEASUREMENT entry_bytes={} capacity={} storage_bytes={}",
            std::mem::size_of::<super::TrailEntry>(), trail.entries.capacity(),
            std::mem::size_of::<super::TrailEntry>() * trail.entries.capacity());
    }
}
RUST
git -C "$repository" show "$head_ref:tests/cleanup_measurement.rs" > "$scratch/probe.rs"
sha256sum "$scratch/layout.rs" "$scratch/probe.rs"
cat "$scratch/layout.rs"

for label in baseline head; do
    ref=$baseline
    [[ $label == baseline ]] || ref=$head_ref
    git -C "$repository" worktree add --detach "$scratch/$label" "$ref"
    checkout=$scratch/$label
    [[ $(git -C "$checkout" rev-parse HEAD) == "$ref" ]]
    [[ -z $(git -C "$checkout" status --porcelain) ]]
    cp "$scratch/probe.rs" "$checkout/tests/cleanup_measurement.rs"
    cat "$scratch/layout.rs" >> "$checkout/src/trail/mod.rs"
    printf 'label=%s ref=%s test-only injection follows\n' "$label" "$ref"
    sha256sum "$checkout/tests/cleanup_measurement.rs"
    git -C "$checkout" diff -- src/trail/mod.rs tests/cleanup_measurement.rs
    (
        cd "$checkout"
        printf '%s\n' 'cargo test --release --features ncolors_6 --test cleanup_measurement --no-run --message-format=json'
        cargo test --release --features ncolors_6 --test cleanup_measurement --no-run --message-format=json > "$scratch/$label-build.json"
        cargo test --release --features ncolors_6 --lib --no-run --message-format=json > "$scratch/$label-lib.json"
    )
done

# Extract executable paths from Cargo's build records; compilation is outside timing.
for label in baseline head; do
    python3 - "$scratch/$label-build.json" > "$scratch/$label-executable" <<'PY'
import json, sys
artifacts = [json.loads(line) for line in open(sys.argv[1])]
paths = [a['executable'] for a in artifacts if a.get('executable') and a.get('target', {}).get('name') == 'cleanup_measurement']
assert len(paths) == 1, paths
print(paths[0])
PY
    (
        cd "$scratch/$label"
        printf 'label=%s layout\n' "$label"
        cargo test --release --features ncolors_6 --lib venn_cleanup_measurement_layout -- --ignored --nocapture --test-threads=1
        printf 'label=%s warmup: cargo test --release --features ncolors_6 --test cleanup_measurement -- --ignored --nocapture --test-threads=1\n' "$label"
        cargo test --release --features ncolors_6 --test cleanup_measurement -- --ignored --nocapture --test-threads=1
    )
done

for repetition in 1 2 3 4 5; do
    order=(baseline head)
    if (( repetition % 2 == 0 )); then order=(head baseline); fi
    for label in "${order[@]}"; do
        executable=$(cat "$scratch/$label-executable")
        printf 'label=%s repetition=%s command=%s --ignored --nocapture --test-threads=1\n' "$label" "$repetition" "$executable"
        (cd "$scratch/$label" && "$executable" --ignored --nocapture --test-threads=1) | tee "$scratch/$label-$repetition.log"
    done
done

python3 - "$scratch" <<'PY'
import pathlib, re, statistics, sys
root = pathlib.Path(sys.argv[1])
medians = {}
for label in ['baseline', 'head']:
    samples = []
    for repetition in range(1, 6):
        values = re.findall(r'VENN_CLEANUP_MEASUREMENT search_ns=(\d+)', (root / f'{label}-{repetition}.log').read_text())
        assert len(values) == 1, values
        samples.append(int(values[0]))
    medians[label] = statistics.median(samples)
    print(f'VENN_CLEANUP_MEASUREMENT {label} samples_ns={samples} median_ns={medians[label]}')
print(f'VENN_CLEANUP_MEASUREMENT head/baseline={medians["head"] / medians["baseline"]:.6f}')
PY
