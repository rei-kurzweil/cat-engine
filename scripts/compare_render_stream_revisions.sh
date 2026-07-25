#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
baseline_revision="${MITTENS_RENDER_BASELINE_REVISION:-65ce0cb}"
current_revision="${MITTENS_RENDER_CURRENT_REVISION:-HEAD}"
profile_instances="${MITTENS_RENDER_PROFILE_INSTANCES:-2048}"
profile_iterations="${MITTENS_RENDER_PROFILE_ITERATIONS:-25}"
comparison_root="${MITTENS_RENDER_COMPARISON_ROOT:-/tmp/mittens-render-stream-comparison}"
target_root="${MITTENS_RENDER_PROFILE_TARGET_ROOT:-/tmp/mittens-render-stream-target}"
probe_path="tests/renderer_optimization_profile.rs"

baseline_worktree="${comparison_root}/baseline"
current_worktree="${comparison_root}/current"
result_dir="${repo_root}/docs/.debug"
timestamp="$(date -u +%Y%m%dT%H%M%SZ)"
result_path="${result_dir}/render-stream-comparison-${timestamp}.txt"

mkdir -p "${comparison_root}" "${target_root}" "${result_dir}"

remove_copied_probe() {
    local path="$1"
    if [[ -f "${path}/${probe_path}" ]] &&
        ! git -C "${path}" ls-files --error-unmatch "${probe_path}" >/dev/null 2>&1; then
        rm -f "${path:?}/${probe_path}"
    fi
}

cleanup() {
    remove_copied_probe "${baseline_worktree}"
    remove_copied_probe "${current_worktree}"
}

trap cleanup EXIT

ensure_worktree() {
    local path="$1"
    local revision="$2"

    if [[ -e "${path}/.git" ]]; then
        remove_copied_probe "${path}"
        git -C "${path}" checkout --detach "${revision}" >/dev/null
    else
        git -C "${repo_root}" worktree add --detach "${path}" "${revision}" >/dev/null
    fi
}

copy_probe() {
    local path="$1"
    mkdir -p "${path}/tests"
    if git -C "${path}" ls-files --error-unmatch "${probe_path}" >/dev/null 2>&1; then
        if ! cmp -s "${repo_root}/${probe_path}" "${path}/${probe_path}"; then
            echo "tracked probe differs in ${path}; choose a revision without the probe or update the comparison harness" >&2
            return 1
        fi
    else
        cp "${repo_root}/${probe_path}" "${path}/${probe_path}"
    fi
}

run_probe() {
    local label="$1"
    local path="$2"
    local target_dir="${target_root}/${label}"

    echo "revision=${label} commit=$(git -C "${path}" rev-parse HEAD)"
    (
        cd "${path}"
        MITTENS_RENDER_PROFILE_INSTANCES="${profile_instances}" \
        MITTENS_RENDER_PROFILE_ITERATIONS="${profile_iterations}" \
        CARGO_TARGET_DIR="${target_dir}" \
            cargo test --release --test renderer_optimization_profile \
            -- --ignored --nocapture --test-threads=1
    ) 2>&1 |
        sed -n '/RENDER_STREAM_PROFILE/{
            s/^.*RENDER_STREAM_PROFILE/RENDER_STREAM_PROFILE/
            p
        }' |
        while IFS= read -r record; do
            payload_bytes="${record##*ordinary_view_stream_payload_bytes=}"
            if [[ "${label}" == "baseline" ]]; then
                copied_bytes="${payload_bytes}"
                copy_allocations=6
            else
                copied_bytes=0
                copy_allocations=0
            fi
            echo "${record} ordinary_view_copied_bytes=${copied_bytes} ordinary_view_copy_allocations=${copy_allocations}"
        done
}

ensure_worktree "${baseline_worktree}" "${baseline_revision}"
ensure_worktree "${current_worktree}" "${current_revision}"
copy_probe "${baseline_worktree}"
copy_probe "${current_worktree}"

{
    echo "render_stream_revision_comparison"
    echo "timestamp_utc=${timestamp}"
    echo "instances=${profile_instances}"
    echo "iterations=${profile_iterations}"
    echo "note=ordinary-view copy fields cover opaque, cutout, and overlay stream selection"
    run_probe baseline "${baseline_worktree}"
    run_probe current "${current_worktree}"
} | tee "${result_path}"

echo "result=${result_path}"
