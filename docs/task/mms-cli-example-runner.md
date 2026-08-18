# Task: CLI runner for live MMS examples

## Problem

An MMS scene under `examples/` currently needs a matching Rust example to
create a `Universe`, evaluate the file with live `World`, `RxWorld`,
`RenderAssets`, and `CommandQueue` references, process emitted intents, and
start `Windowing`.  The shared `examples/mms_live_launcher.inc` removes most
of that repetition, but every new `.mms` file still requires a Rust target such
as `examples/combine-mesh.rs`.

This is useful as a temporary explicit bootstrap, but it does not scale to a
library of MMS examples and makes ad-hoc scene authoring unnecessarily slow.

## Objective

Provide one CLI command that evaluates an MMS file into a live engine universe
and opens it in Mittens.

```text
cargo run --bin mittens-mms -- examples/combine-mesh.mms
# or, after installation:
mittens-mms examples/combine-mesh.mms
```

The command must use the same live-world path as a Rust example, not the
evaluation-only `MeowMeowRunner::eval_file` helper.  Imports must resolve
relative to the supplied scene path.

## Proposed behavior

1. Parse CLI arguments: required MMS path; optional window size, working asset
   root, REPL enablement, and a no-window/validate-only mode.
2. Resolve the scene path before evaluation and make its parent available as
   the import base.
3. Call `example_support::ensure_model_assets()` and initialize logging.
4. Create `Universe::new(World::default())`.
5. Evaluate using `MeowMeowRunner::eval_with_world_and_assets_at_path`, passing
   the universe's live world, RX world, render assets, and command queue.
6. Print every MMS diagnostic with the scene path and exit non-zero on errors;
   never launch a partially evaluated scene.
7. Push emitted intents into the universe queue, call
   `systems.process_commands`, optionally enable the REPL, then call
   `Windowing::run_app`.

The initial command should deliberately mirror `mms_live_launcher.inc` so the
move is behavior-preserving.  Once it exists, small Rust wrapper examples can
be retired incrementally; keep wrappers where they add custom native setup.

## CLI shape

```text
mittens-mms <scene.mms> [--width <px>] [--height <px>]
                        [--repl] [--validate-only] [--asset-root <path>]
```

- `--validate-only` performs live-world evaluation and command processing but
  does not create a window.  It is suitable for CI smoke tests.
- Width/height should override `RendererSettings.window_size` only if the
  renderer exposes a clean pre-window override; otherwise defer those flags.
- `--asset-root` is only needed if model/texture discovery cannot reliably
  start from the repository root.  Do not change the process working directory
  implicitly.

## Design notes

- Reuse a library-level `launch_mms_scene(path, LaunchOptions)` helper from
  both the CLI and any remaining wrapper examples.  The helper returns an
  error before window startup so tests can assert failures without a process
  exit.
- Preserve `include_str!` wrappers for examples that intentionally compile a
  scene into the binary.  The CLI reads from disk and is the appropriate path
  for iterative authoring.
- Runtime imports must retain the original canonical/relative source path;
  passing only a source string loses the module-relative import base.
- A future `--watch` mode may rebuild the MMS world on file changes, but it is
  out of scope for the first runner.

## Acceptance criteria

- `mittens-mms examples/combine-mesh.mms` displays the same scene as
  `cargo run --example combine-mesh`.
- Relative imports of `assets/components/truss.mms` and the kawaii background
  resolve without scene-specific Rust code.
- MMS evaluation errors are readable and return a failing status before a
  window is launched.
- `--validate-only examples/combine-mesh.mms` is runnable in CI and confirms
  the live-world command path rather than merely parsing the file.
- At least one existing wrapper migrates to the shared helper as a regression
  test for parity.
