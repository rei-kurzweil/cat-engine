# Paint-stroke live diagnostics

Use the diagnostic scene to observe the existing Free Draw, Spray Can, and
Line input pipelines without changing their behavior:

```sh
MITTENS_DEBUG_PAINT_STROKE=1 cargo run --example paint-stroke-debug
```

To retain a trace for comparison:

```sh
MITTENS_DEBUG_PAINT_STROKE=1 cargo run --example paint-stroke-debug 2>&1 | tee /tmp/mittens-paint-stroke.log
```

The environment flag is opt-in. Without it, the extra markers and
`paint_stroke_trace` records are disabled.

For the related visual/snap-frame mismatch, see the
[Grid visual-coordinate-space tracker](../task/grid-visual-coordinate-space-tracker.md).
The diagnostic scene is its reproduction fixture; that rendering task does not
change Paint's current snap-source behavior.

## Scene and marker legend

The scene contains three adjacent vertical paint targets, a shelf, a floor,
one vertical grid, one translated/rotated floor grid, editor panels, a desktop
pointer, and an XR pointer rig. Use the Grid panel to choose a grid and the
Assets and Paint panels to choose the placement operand and tool.

While a stroke is active, small non-raycastable, nonselectable, nonserialized
spheres show:

- green: gesture start point;
- magenta: point mapped by the gesture system and delivered to Paint;
- cyan: that point projected/snapped through the editor-selected grid; and
- yellow: snapping resolved through Paint's current hit-owned-grid path.

A missing cyan marker means there is no selected enabled grid. A missing
yellow marker is expected when the pointer hits ordinary scene geometry:
committed grid visuals are currently non-raycastable, so the existing Paint
path normally cannot discover their grid from the hit renderable. Markers are
removed when the stroke ends. They are diagnostic only and never become scene
content.

## Trace records

Every record begins with `paint_stroke_trace`. The `gesture` layer reports
start, move, end, and pointer/raycaster-disappearance cancellation, including:

- pointer class and raycaster;
- captured versus live renderable;
- raw ray origin/direction and live hit point; and
- the mapping policy and point delivered downstream.

The `paint` layer reports the normalized event, selected editor/tool/stroke,
captured renderable, mapped point, preview/effect lifecycle, and two grid
diagnostics. `selected_grid` is the editor-context grid; `actual_paint_grid` is
the grid inferred by the current Paint implementation from hit ownership.
Each available diagnostic includes grid owner, spacing, grid-local point,
integer address, and snapped world point.

If preview startup fails, `phase=preview_start_failed` names the first failing
stage: selected asset, asset spawn, asset bounds, surface frame, or placement
pose. This is emitted only when diagnostics are enabled and does not change
the failed gesture's behavior.

Filter a retained log with:

```sh
rg 'paint_stroke_trace' /tmp/mittens-paint-stroke.log
```

## Investigation passes

Run each pass first with the desktop pointer and then, where available, with
the XR pointer:

1. With no active grid, use Free Draw for a click, a slow horizontal drag
   across all three wall targets, a fast version of that drag, and a drag that
   leaves the target.
2. Select the vertical grid and repeat. Compare magenta with cyan, and compare
   `selected_grid` with `actual_paint_grid` in the trace.
3. Repeat Free Draw on the floor with the translated/rotated grid selected.
4. Repeat the wall passes with Spray Can. Record where placements occur versus
   the mapped and snapped markers.
5. Select Line and repeat a click and drag. The current no-placement result is
   a known baseline; the trace should still expose the gesture and paint
   lifecycle.
6. If possible, remove a pointer or raycaster during a stroke. The gesture
   trace currently records cancellation, while Paint has no cancellation event
   and may retain its runtime/preview. That mismatch is part of the
   investigation.

The useful comparison is not only whether an object appears. Record the first
phase at which expected and observed data diverge: raw ray/live hit, gesture
mapping, paint event delivery, selected-grid projection, current hit-grid
resolution, preview update, or commit.
