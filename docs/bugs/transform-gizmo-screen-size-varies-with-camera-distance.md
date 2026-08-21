# Transform gizmo screen size varies with camera distance on desktop

Date: 2026-07-28

Status: open / desktop confirmed / VR behavior acceptable in recent check

Related:

- `docs/spec/transform-camera-specific.md`
- `docs/spec/screen-space-lines.md`
- `docs/task/grid-gizmo-paint-end-to-end-ux-and-test-matrix.md`
- `docs/task/gizmo-drag-regression-and-lock-toggle.md`
- `docs/task/render-stream-single-source.md`
- `docs/spec/mesh-deformation-pipeline.md`
- `docs/bugs/xr-hand-laser-is-selectable-and-origin-is-past-fingertip.md`

## Summary

Transform gizmos no longer appear approximately constant in screen/angular size
in the desktop view.

Observed in `bisket-vr-demo`:

- a gizmo looks larger as the desktop camera gets closer
- it looks smaller as the camera gets farther away
- gizmos can appear to have different sizes depending on when and where a scene
  target is clicked
- this is currently confirmed on desktop
- in a recent VR check, gizmos still vary somewhat with angle and distance but
  look acceptable; the strong desktop regression is not reproduced there

That is the ordinary perspective-size behavior expected when the
camera-dependent compensation scale is missing, stale, or not reaching the
rendered instances.

This report is separate from grid snapping. The terrain currently begins with
cell edges aligned to whole-unit grid boundaries, while the first snapped gizmo
move changes the cube to a half-cell X/Z phase. Both issues are visible through
the gizmo, but they have different implementation paths.

## Expected behavior

For a perspective desktop camera:

- the gizmo should retain approximately the same angular/screen extent as camera
  distance changes
- selecting an equivalent target at the same depth should produce the same
  gizmo size
- changing selection should not reset the gizmo to an arbitrary stale scale
- visual geometry and raycast/BVH handle bounds should use the same effective
  scale in the same frame

For XR:

- both eyes should use the current cyclopean approximation consistently
- moving the head toward or away from the gizmo should not produce ordinary
  perspective growth/shrinkage

Exact constant pixel width for line thickness is outside the current contract.
The whole gizmo should nevertheless remain approximately constant in angular
size.

## Current implementation contract

This behavior is not currently implemented in a vertex shader.

`TransformGizmoSystem::update_camera_scales(...)` runs after window/XR cameras
publish their matrices and before raycasting. It:

1. resolves the gizmo's `visual_root`
2. reads the visual root world position
3. selects XR eye views when stereo is active, otherwise the window view
4. computes positive camera-space depth
5. computes:

   ```text
   scale = gizmo.scale * depth / 4.0
   ```

6. clamps scale to `[0.02, 20.0]`
7. writes the selected mono/stereo settings `TransformComponent`
8. asks transform propagation to refresh the camera-specific anchor
9. refits BVH bounds before pointer raycasts

`TransformCameraSpecific` then evaluates:

```text
effective_world = generic_anchor_world * selected_settings_local
```

Gizmo handles use ordinary unskinned `TOON_MESH` renderables under an overlay
subtree. They do not use the cached-skinned toon vertex shader.

### XR-active desktop diagnosis

The camera-scale path deliberately gives XR precedence whenever an active XR
rig has published eye views:

```text
stereo_active = active_xr_camera && xr camera has eyes
TransformCameraSpecific = Stereoscopic
```

It does this even when a desktop/window camera is also rendering. Therefore a
scene such as `paint-stroke-debug`, which enables `XR.on()` and an XR pointer
rig, is not a clean desktop gizmo-size reproduction: its gizmo scale is derived
from XR eye depth and its stereoscopic settings transform. A desktop view can
then display a size that does not track the window camera's distance.

This is a confirmed explanation for that scene's mixed desktop/XR behavior,
not yet a decision that a simultaneously active XR session should use a
different policy. Use `paint-grids-desktop` for a desktop-only grid/paint
reproduction and test gizmo sizing there before changing the camera-selection
contract.

Relevant code:

- `src/engine/ecs/system/gizmo_system.rs`
- `src/engine/ecs/system/transform_stream_system.rs`
- `src/engine/ecs/system/transform_system.rs`
- `src/engine/ecs/system/system_world.rs`
- `src/engine/graphics/visual_world.rs`
- `src/engine/graphics/vulkano_renderer.rs`
- `examples/paint-grids-desktop.mms`

## Why the observed behavior indicates a missing/stale compensation

In perspective projection, an unadjusted world-space object becomes:

- larger on screen when closer
- smaller on screen when farther away

The current compensation is intended to multiply gizmo world scale by depth,
which cancels that relationship approximately. The reported direction of change
therefore suggests one of:

- the camera scale update is skipped
- the wrong camera/view is used
- the settings transform changes but is not selected or propagated
- ECS/VisualWorld has the new matrix but the renderer uses stale instance data

It does not look like a simple wrong constant. A wrong constant would make every
gizmo consistently too large or too small while still remaining stable with
distance.

## Likely regression seams, in investigation order

### 1. Camera scale update is skipped or lacks a valid window camera

Check:

- shared workspace gizmo is present in `live_gizmos`
- `TransformGizmoComponent.visual_root` is populated
- `CameraSystem::has_active_window_camera()` is true
- `VisualWorld::visual_camera(CameraTarget::Window)` has an eye/view
- computed depth is finite and positive

If any condition fails, `update_camera_scales(...)` leaves the last settings
scale unchanged.

### 2. Shared-gizmo selection lifecycle leaves stale state

The editor now uses a shared workspace gizmo which is re-targeted across scene
selections. The “depends on when/where I click” symptom could mean:

- multiple gizmos still exist unexpectedly
- the shared gizmo is recreated without entering `live_gizmos`
- target/anchor changes do not immediately refresh camera scale
- a newly selected target is rendered for one or more frames using the initial
  `0.5` settings scale

Inventory the number and identities of:

- `editor_transform_gizmo`
- `gizmo_root`
- mono/stereo settings transforms

before and after repeated selections.

### 3. `TransformCameraSpecific` effective matrix is not reaching descendants

The settings scale may be correct while:

- the retained pre-camera basis is stale
- a direct refresh starts below the transform-stream boundary
- downstream overlay transforms reconstruct from authored TRS and bypass the
  camera-specific effective matrix
- repeated selection/refresh changes the boundary topology

Existing tests cover mode selection and non-compounding evaluation in isolation.
They do not cover a live gizmo, camera movement, VisualWorld propagation, and
render submission end to end.

### 4. VisualWorld or cached overlay instance data is stale

`VisualWorld::update_model(...)` sets `dirty_instance_data`, and the renderer is
expected to rebuild its cached overlay instance buffer when that flag is
consumed.

The render-stream cache path changed substantially on 2026-07-24. Verify:

- a camera-scale change calls `VisualWorld::update_model(...)` for every gizmo
  handle renderable
- `dirty_instance_data` is still true when the relevant render view consumes it
- the cached overlay instance buffer is rebuilt
- the buffer contains the latest gizmo model matrices
- multiple render views/eyes do not consume the dirty state before the desktop
  view receives updated instance data

### 5. Shared renderer/deformation changes altered instance submission

The 2026-07-26 cached deformation work changed renderer instance submission and
descriptor paths. The gizmo itself is unskinned, so the deformation shader is
not the direct scale mechanism. Still verify that:

- ordinary `TOON_MESH` overlay instances use the expected instance layout
- their model matrix comes from the ordinary instance buffer
- mixed skinned/unskinned scenes do not select the wrong buffer range or stale
  cached data

Treat this as a renderer integration hypothesis, not the leading mathematical
hypothesis.

## Recent history worth comparing

- 2026-07-14, `ab2ede4`: introduced `TransformCameraSpecific` and
  depth-compensated gizmo scale
- 2026-07-24, `5fc985b`: removed legacy foreground draw batches and made
  window/XR views use cached render streams/instance buffers
- 2026-07-26, `ef592dc`: introduced cached compute deformation and substantial
  renderer submission changes
- 2026-07-27, `069c4a6` and `c036271`: changed render-view/XR submission
  sequencing

These commits define comparison points; they do not yet establish causality.

## Required diagnostic trace

Add one rate-limited row per active gizmo with:

```text
frame
gizmo id
target id
visual_root id
camera family
window camera present
camera/view id
anchor world position
camera-space depth
requested settings scale
stored mono scale
stored stereo scale
effective anchor world scale
representative handle VisualWorld model scale
dirty_instance_data
overlay instance buffer rebuilt
```

Capture the row:

- immediately after selecting a near target
- after moving the camera without changing selection
- immediately after selecting a far target
- after moving back toward the target

This distinguishes calculation, propagation, and GPU-cache failures without
guessing from the final image.

## Automated coverage needed

### System-level constant-angular-size test

Build a live gizmo fixture with a perspective window camera.

For target depths `2`, `4`, and `8`:

1. run camera update
2. run `update_camera_scales(...)`
3. propagate the returned anchor
4. inspect a representative gizmo handle's VisualWorld model
5. project its endpoints through the same view/projection
6. assert approximately equal NDC/screen extent

This must exercise the live gizmo topology rather than only testing the scale
formula.

### Selection retarget test

With one shared workspace gizmo:

1. select a near target
2. record gizmo id and projected extent
3. select a far target
4. assert the same gizmo is retargeted, or explicitly assert the intended
   replacement lifecycle
5. assert projected extent remains stable on the first settled frame
6. repeat near/far selection several times

### VisualWorld dirty/cache test

After changing only camera-derived gizmo scale:

- representative handle model matrix changes
- `dirty_instance_data` is raised
- the overlay instance data rebuilt for the next desktop render contains the new
  model

### Desktop/XR parity

Run equivalent distance changes with:

- desktop window camera only
- XR camera active
- XR plus desktop mirror/window

Record which family owns the single effective transform in the combined case.

## Manual verification matrix

Use `bisket-vr-demo`.

| ID | Mode | Action | Expected | Current |
|---|---|---|---|---|
| GS-01 | desktop | Select a target about 1–2 m away | Baseline gizmo screen extent | Verify numerically/screenshot |
| GS-02 | desktop | Move camera to roughly twice the distance | Same approximate screen extent | Confirmed fail: gizmo appears smaller |
| GS-03 | desktop | Move closer than the baseline | Same approximate screen extent | Confirmed fail: gizmo appears larger |
| GS-04 | desktop | Alternate selecting near and far targets | Stable extent on every settled selection | Confirmed inconsistent by click location/time |
| GS-05 | desktop | Keep selection and orbit without changing depth | Stable extent | Verify |
| GS-06 | desktop | Resize the window | Stable angular size; aspect updates correctly | Verify |
| GS-07 | XR | Move head toward/away from selected target | Stable approximate angular extent | Acceptable in recent VR check; some angle/distance variation remains |
| GS-08 | XR + window | Compare headset and desktop views | Behavior matches documented single-family policy | Not recently verified |
| GS-09 | picking | Click/drag handles at near and far distances | Visible and interactive bounds agree | Verify |

For screenshots, keep viewport size fixed and record:

- target/gizmo world position
- camera world position
- camera-space depth
- projected gizmo bounding-box width/height in pixels

## Fix order

1. Add the diagnostic trace.
2. Add the system-level near/far projected-extent test.
3. Verify camera availability and `live_gizmos`/`visual_root`.
4. Verify selected settings and transform-stream propagation.
5. Verify VisualWorld model changes and overlay dirty-buffer rebuild.
6. Only then inspect shader/descriptor variants if CPU and instance-buffer
   matrices are correct.
7. Run desktop verification.
8. Run XR and combined-view verification.

## Definition of done

- desktop near/far projected extent remains within an agreed tolerance
- repeated selection does not produce size-dependent first frames or stale scale
- visible geometry and raycast bounds agree
- window resize does not break compensation
- XR behavior is verified and documented
- combined XR/window behavior matches the single-effective-transform policy
- an automated end-to-end regression test covers camera movement and selection
  retargeting
