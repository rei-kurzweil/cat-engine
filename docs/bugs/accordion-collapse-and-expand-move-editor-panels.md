# Collapsing and expanding editor panels changes their positions

Date: 2026-09-05

Status: open; observed across multiple accordion-backed editor panels

## Summary

Collapsing or expanding an editor panel moves the panel in world space even though the user did not
drag it. The collapsed and expanded states settle at two different positions, and the particular
positions appear to be unique to each panel.

This has been observed on the Editor Settings, Color, and Paint panels and may affect every editor
panel that uses the shared accordion shell.

## Reproduction

1. Load a scene with one or more editor panels expanded.
2. Note the position of a panel, especially a stable point on its title bar.
3. Click the panel's accordion control to collapse it.
4. Observe that the panel moves to a different position.
5. Click the control again to expand the panel.
6. Observe that the panel moves again, to a position different from its collapsed position.
7. Repeat with the Editor Settings, Color, and Paint panels.
8. Check the remaining accordion-backed editor panels for the same behavior.

For diagnosis, record the local and world transforms of the outer layout slot, named panel root,
private `LayoutRoot`, and title bar before and after each transition. This should distinguish a real
panel-root transform change from descendant layout content moving within an otherwise stable root.

## Expected behavior

- Collapsing a panel removes or suspends its body while leaving the panel's anchor position
  unchanged.
- Expanding a panel restores its body at that same anchor position.
- The title bar remains stationary across both transitions.
- Repeated collapse/expand cycles do not introduce positional drift or alternate between different
  world-space placements.
- Panels only move in response to an explicit drag or another intentional workspace-layout action.

## Actual behavior

- Collapsing a panel moves it without a drag.
- Expanding it moves it again.
- The collapsed and expanded states occupy two different positions.
- Different panels appear to use different state-dependent positions.
- The issue is shared by at least Editor Settings, Color, and Paint rather than being specific to
  one panel body implementation.

## Investigation targets

- The shared accordion topology in `assets/components/internal/ui/accordion.mms`:
  - the outer `accordion_layout_slot`
  - the stable named panel root
  - the private `LayoutRoot`
  - removal and restoration of `#accordion_body`
- The accordion event handling and body restoration path in
  `src/engine/ecs/system/editor_inspector_system_stopgap_mms_adapter.rs`.
- Shared editor workspace relayout after a panel's computed height changes.
- Whether layout-owned transforms overwrite an authored or dragged panel transform when the body
  is removed or reattached.
- Whether the layout origin, centering, or post-layout bounds adjustment depends on the current
  expanded height and therefore shifts the title bar.
- Whether a restored body dirties a different layout root or causes a second placement pass with a
  different coordinate basis.

The stable panel shell should own the persistent anchor. Expanded body height is layout output and
must not be allowed to change that anchor merely because the body exists or does not exist.

## Acceptance criteria

- [ ] The outer panel anchor and title-bar world position remain unchanged when collapsing.
- [ ] The same positions remain unchanged when expanding.
- [ ] The invariant holds for Editor Settings, Color, Paint, Pose, World, Inspector, Assets, and
      Grid panels.
- [ ] The invariant holds after a panel has been dragged to a non-default position.
- [ ] At least 100 collapse/expand cycles produce no drift.
- [ ] Restoring still reconstructs current body content and correct layout dimensions.
- [ ] Panel focus, dragging, selection, scrolling, and controls continue to work after restoration.

## Related

- [Accordion panel restore loses content and corrupts backgrounds](./accordion-panel-restore-content-and-background-corruption.md)
- [Editor panel minimize and render suspension](../task/editor-panel-minimize-and-render-suspension.md)
- [Layout root computed size and shift event](../task/layout-root-computed-size-and-shift-event.md)
