# Task: World-panel topology refresh for `CombineMesh`

## Why

`CombineMesh {}` now removes its baked source subtrees in the default mode,
leaving one root/output representation.  The live `World` therefore has the
right shape, but an already-open World panel can continue to show its cached
rows from before the bake.  That makes it appear that the individual truss
bars still exist and obscures the distinction between default collapse and
`CombineMesh.keep_transforms()`.

The immediate work is to refresh only the World panel when relevant authored
topology changes.  This is an editor projection problem, not a reason to
refresh every editor panel or to add renderer statistics first.

The combine-mesh example should also omit the Asset panel.  Its long,
unclipped asset list is unnecessary for this scene and costs work until editor
scrolling and clipping are improved.

## Existing seams

- `WorldPanel` already owns an `AuthoredWorldPanelSceneModel`, built with
  `build_authored_world_panel_scene_model(...)` and replaced through
  `rebuild_world_panel_scene_model(...)`.
- `rerender_world_panel_for_context(...)` rebuilds that model and rerenders
  only the World panel content/status.  Settings, Inspector, Asset, and other
  panels have separate refresh paths.
- `EditorUI` accepts an explicit `panels([...])` list.  If `assets` is absent,
  the panel layout does not create `#assets_root`; `populate_asset_panel(...)`
  consequently exits without building asset rows.
- Normal authored parent changes already have editor command/lifecycle seams.
  Default CombineMesh collapse currently happens later in render preparation
  through immediate subtree removal, so it needs to notify the same scoped
  World-panel refresh path explicitly rather than relying on a cached model.

## Phase 1: scoped World-panel invalidation

Introduce one coalesced "authored topology changed" signal/dirty flag for the
editor workspace.

### Criteria for marking it dirty

Mark an editor root dirty only when a structural mutation affects its authored
scene subtree:

- attach, detach, reparent, or subtree removal below an installed/effective
  editor root;
- a default `CombineMesh` source collapse below that root, after the combined
  output has registered successfully;
- editor-root installation/removal itself.

Do not mark it for transform, material, or ordinary property changes.  Those
do not alter the World-panel tree.  Do not mark it for editor UI helper
subtrees, gizmos, or unrelated runtime trees outside effective editor roots.

The test must define the root check precisely: before removal, use the source
root's current ancestor chain; for an attach/reparent, consider both the old
and new authored ancestry.  A CombineMesh system removal should pass its known
source root to this helper before/while removing it, because the ancestor link
will no longer exist afterward.

### Flush behavior

At a safe editor update boundary, process each dirty editor root at most once
per frame/command flush:

1. rebuild `AuthoredWorldPanelSceneModel` from the live world;
2. rerender the World panel via its existing context-aware function;
3. retain selection only if its target still exists; otherwise clear/fall back
   using the panel's existing selection behavior.

This must not rerender Settings, Inspector, Asset, Grid, Paint, or Color
panels merely because the World tree changed.  It must also avoid recursively
dirtying itself while it constructs the World-panel UI subtree.

## Phase 2: make the combine-mesh scene lightweight

Change `examples/combine-mesh.mms` to request only the panels used to inspect
this example:

```mms
EditorUI {
  panels([
    { panel = "world" },
    { panel = "settings" },
  ])
}
```

This is intentionally an explicit scene configuration, not a global change to
the default `EditorUI {}` panel set.  It prevents the Asset panel from being
mounted and from populating its asset-list content for this heavy example.

## Phase 3: deferred diagnostics and editor list performance

An `EngineStatsPanel` / `VisualWorld::stats_snapshot()` is useful future work,
but is not required to establish that CombineMesh collapsed source topology.
Revisit it after the World panel has correct live rows and after scrolling,
clipping, and virtualized-list behavior have a clear performance plan.

When revisited, renderer diagnostics should remain graphics-owned,
snapshot-based, internally scrollable, and use `overflow("hidden")` on the
panel shell.  They should report live VisualWorld instance/batch/stream data,
not infer renderer work from authored World rows.

## Acceptance criteria

- After a successful default CombineMesh bake/collapse, the open World panel
  refreshes to show the short collapsed hierarchy; individual truss bars are
  absent from both the live World and its displayed rows.
- `CombineMesh.keep_transforms()` retains its source rows and remains the mode
  where source-transform editing causes a rebake.
- A topology change outside effective editor roots, or an editor helper/gizmo
  tree change, does not rerender the World panel.
- Multiple source removals from one CombineMesh collapse cause no more than
  one World-panel model rebuild/rerender for that editor update boundary.
- The combine-mesh example mounts World and Settings but no Asset panel; the
  asset-panel population path has no work to do for that scene.
- Existing panel-specific refresh behavior remains unchanged.
