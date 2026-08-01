# Move Meow Meow documentation into its crate

Date: 2026-08-01

Status: active tracker

Make `crates/meow-meow-script/docs` the canonical home for host-neutral MMS
documentation. Keep only Mittens integration, migration, and engine-specific
material under the workspace-level `docs` tree.

## Tracker

- [x] Create `crates/meow-meow-script/docs/how_to`.
- [x] Add a current guide for configuring a script host.
- [x] Add a current guide for using and inspecting `StandardHost` from MMS and
      Rust.
- [x] Link the crate README, workspace README, and existing MMS overview to
      the crate-local how-to guides.
- [ ] Inventory `docs/meow_meow` and classify each page as language/crate,
      Mittens integration, historical analysis, or obsolete.
- [ ] Move host-neutral language specifications, drafts, analyses, roadmaps,
      and tasks under `crates/meow-meow-script/docs`, preserving useful
      category directories.
- [ ] Keep the Mittens host/runtime boundary and engine adapter documentation
      in workspace-level docs, linking outward to crate-owned language docs.
- [ ] Update all workspace Markdown links after the moves; do not leave both
      locations claiming to be canonical.
- [ ] Update crate-local links so they remain valid when the crate is packaged
      independently of the Mittens repository.
- [ ] Add or run a Markdown link check over both documentation roots.
- [ ] Verify the crate package includes its README, guides, and referenced
      examples.

## Initial crate guides

- [Configuring a script host](../../crates/meow-meow-script/docs/how_to/configuring_a_script_host.md)
- [Use the standard host](../../crates/meow-meow-script/docs/how_to/use_the_standard_host.md)

