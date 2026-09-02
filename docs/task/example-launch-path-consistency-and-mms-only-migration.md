# Example launch-path consistency and MMS-only migration

Date: 2026-09-01
Status: open

## Problem

Examples can currently launch through materially different paths:

- `cargo run --release -- load examples/<scene>.mms` loads a filesystem-backed
  MMS source and resolves imports relative to that source file.
- `cargo run --release --example <name>` may embed source with `include_str!`
  and evaluate without a source path, causing relative imports to resolve from
  the process working directory instead.

`examples/vtuber-desktop.mms` demonstrates the inconsistency: its import of
`assets/components/poses/bisket/000-relaxed.pose.mms` succeeds for the embedded
example path but resolves incorrectly as `examples/assets/...` through the CLI
loader. The file-relative spelling must be `../assets/...`.

## Goal

Every maintained example has one documented canonical launch command and
behaves consistently when loaded from an MMS file. Reduce duplicated Rust
wrappers where MMS plus the general CLI fully expresses the example.

## Inventory and classification

For every `examples/*.mms` and corresponding `examples/*.rs`, record:

| Class | Meaning | Direction |
| --- | --- | --- |
| MMS-only candidate | No Rust-only setup, diagnostics, or APIs required | Keep `.mms`; remove or retire the thin wrapper |
| Wrapper required | Requires native asset setup, programmatic visual setup, test harness, or unavailable MMS feature | Keep wrapper; evaluate source with its filesystem path |
| Deliberate dual path | Both entry points have distinct supported purposes | Document both and test import behavior |

## Work items

- [ ] Fix every filesystem-loaded example import to be relative to its `.mms`
  file, never implicitly relative to repository CWD.
- [ ] Change retained wrappers from pathless `eval(include_str!(...))` to the
  path-aware evaluator or otherwise supply the source path.
- [ ] Add a smoke test that materializes every maintained filesystem MMS
  example with its real source path.
- [ ] Inventory pairs and label each as MMS-only candidate, wrapper required,
  or deliberate dual path.
- [ ] Remove thin wrappers only after the CLI launch path is validated and the
  canonical command is documented.
- [ ] Update contributor/example documentation with canonical launch commands.

## Non-goals

- Replacing Rust examples that intentionally exercise native-only APIs.
- Changing MMS module semantics to search arbitrary parent directories.
- Treating process CWD as a fallback for relative module imports.

## Initial evidence

- `cargo run --release -- load examples/vtuber-desktop.mms` fails because the
  scene's `assets/...` import resolves under `examples/`.
- `cargo run --release --example vtuber-desktop` opens because its wrapper
  embeds MMS with `include_str!` and does not supply a source path.

## Related

- `examples/vtuber-desktop.mms`
- `examples/vtuber-desktop.rs`
- `src/scripting/runner.rs`
