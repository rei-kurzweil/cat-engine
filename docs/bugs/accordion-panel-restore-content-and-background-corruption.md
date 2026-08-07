# Accordion panel restore loses content and corrupts backgrounds

Date: 2026-08-06

Status: open `mittens-engine 0.8.0` release regression; reproduction and root cause pending

## Summary

Editor panels can be minimized through the authored accordion button, but the
restore path is not reliable:

1. after minimizing and restoring the Asset panel, its body contents do not
   render; and
2. after restoring the World or Asset panel, a very large bright
   emissive-yellow background quad appears, or an existing background quad is
   resized far beyond the intended panel bounds.

Treat these as related restoration-lifecycle symptoms until evidence proves
they have different causes. They remain separate acceptance failures so fixing
one cannot hide the other.

## Reproduction to confirm

1. Start an editor example with the World and Asset panels expanded.
2. Record the panels' visible contents, background bounds, body topology,
   renderables, layout roots, and dynamic slot registrations.
3. Minimize the Asset panel with its accordion control.
4. Restore the Asset panel.
5. Confirm whether its restored body and rows exist in ECS, layout output, and
   render registrations even when they are not visible.
6. Repeat minimize/restore for the World panel.
7. Identify the component and render instance responsible for any oversized
   emissive-yellow quad.
8. Repeat both cycles several times and record whether nodes, renderables,
   handlers, backgrounds, or slot registrations accumulate.

Record the exact example, desktop/XR path, renderer settings, and whether bloom
only amplifies the symptom or the underlying quad itself is incorrectly sized.

## Expected behavior

- Restore materializes exactly one fresh `#accordion_body` below the stable
  body mount.
- Every body-owned selector, control, data-renderer slot, layout node, and
  render registration is resolved from the new component IDs.
- The controller refreshes the body exactly once from current model state.
- Asset and World content renders immediately after restoration.
- Panel background geometry matches the restored layout bounds and retains the
  intended material/color/emissive values.
- Repeated cycles have stable expanded/minimized counts and dimensions.

## Actual behavior

- Asset-panel body content is missing visually after restoration.
- Restored World and Asset panels can show an oversized bright
  emissive-yellow background quad.
- It is not yet known whether the second symptom is a duplicate background, a
  stale renderer/layout registration, incorrect restored topology, or an
  existing background resized using stale layout state.

## Investigation checklist

- [ ] Capture a minimal deterministic reproduction and before/after topology.
- [ ] Distinguish missing ECS content from missing layout, visibility, stencil,
      data-slot projection, transform-stream, and renderer registration.
- [ ] Verify the restore handler does not reuse removed body/slot component IDs.
- [ ] Verify `DataRendererSystem` forgets removed slots and binds the fresh
      restored slots exactly once.
- [ ] Trace layout background ownership and identify the exact yellow quad's
      component, material, emissive value, and calculated bounds.
- [ ] Check for duplicate `Style`/background descendants or a body attached at
      the wrong mount/root.
- [ ] Check whether stale layout or render handles survive `RemoveSubtree`.
- [ ] Compare first restore with later cycles for accumulating state.
- [ ] Determine whether World and Asset share one failing lifecycle helper.

## Acceptance

- [ ] Asset contents render after the first and every subsequent restore.
- [ ] World contents render after the first and every subsequent restore.
- [ ] Neither panel creates or enlarges an unintended background quad.
- [ ] Expanded background bounds match the panel layout after every cycle.
- [ ] At least 100 minimize/restore cycles retain stable ECS node, handler,
      renderable, background, and data-slot counts.
- [ ] Model changes while minimized appear after one restoration refresh.
- [ ] Focus, dragging, selection, scrolling, clipping, and panel controls still
      work after restoration.
- [ ] Focused automated coverage prevents both symptoms from recurring.

## Related documents

- [Editor panel minimize and render suspension](../task/editor-panel-minimize-and-render-suspension.md)
- [Mittens 0.8 and 0.9 release roadmap](../task/release-roadmap-0.8.0-0.9.0.md)
- [Panel frame size notes](../analysis/panel-frame-size-notes.md)
- [Panel stencil geometry](panel-stencil-geometry.md)

