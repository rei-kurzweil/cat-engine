# Grid startup visibility and UI state are out of sync

## Status: closed — user validated 2026-09-06

After additional fixes, the user tested `examples/vtuber-mirror-example.mms`,
reported grid visibility working, and requested closure for now. This closes
the reported default-grid visibility/UI mismatch based on that interactive
validation. Reopen if it recurs. The earlier desktop-example change and model
test alone were not evidence of a fix across examples.

## Investigation history

2026-09-06: the user clarified the intended behavior: keep the initial grid
hidden, show Hidden in the panel, and make the first visibility toggle show it.
The desktop example now explicitly authors `hidden(true)`. The generated editor
default already uses `GridSpawnSpec::default_hidden_editor_grid()` with
`hidden=true`; no global Grid constructor or explicitly visible scene state is
changed.

Source inspection confirms the panel derives both its model and row label from
`!entry.hidden`, and its action calls `toggle_grid_hidden` on the component.
The example previously authored `hidden(false)` but lacked live runtime at
startup. Thus its Visible label reflected authored state, while the absent
runtime made it look hidden. The first click changed that state to hidden and
created hidden runtime; the second showed it. Starting this example with the
intended authoritative hidden state removes that extra toggle.

Regression `startup_grids_are_hidden_and_show_on_first_toggle` covers the actual
MMS load and the generated editor default: initially hidden panel model, first
toggle establishing visible live opacity, and second toggle hiding the same
runtime. At that stage, interactive validation was still pending; see the
subsequent user validation above.

The earlier audit also identified an initialization gap for explicitly visible
authored grids. That separate case was not revalidated during this closure;
the hidden startup policy change did not implement a general lifecycle repair.

The empty-grid registration fix was confirmed working by the user before this
visibility work; its captured-grid-plane continuation remains separate.

## Earlier default-grid report

The sections below retain the earlier intentionally-hidden-default reproduction.
Its suspected stale-UI explanation is not established for the authored-visible
case above; investigate each state's initialization explicitly.

## Summary

When a scene using the editor first loads, the default grid is correctly hidden in the scene, but
the grid panel initially labels it as `Visible`.

Because the UI state does not match the grid's actual visibility, showing the grid requires two
clicks: first to change the control to `Hidden`, then again to change it to `Visible` and make the
grid appear.

## Repro

1. Load a scene that uses the editor.
2. Observe the default grid immediately after startup.
3. Confirm that the grid is not visible in the scene.
4. Observe that its visibility control says `Visible`.
5. Click the visibility control once.
6. Observe that the control changes to `Hidden`, while the grid remains hidden.
7. Click the visibility control a second time.
8. Observe that the control changes to `Visible` and the grid finally appears.

## Expected behavior

- The default grid should remain hidden on startup.
- Its visibility control should initialize to `Hidden` to match the actual grid state.
- Clicking the control once should change the state to `Visible` and show the grid.

## Actual behavior

- The default grid is hidden on startup.
- Its visibility control incorrectly initializes to `Visible`.
- The first click only brings the UI into sync by changing it to `Hidden`.
- A second click is required to set the grid to `Visible` and render it.

## Suspected problem areas

This appears to expose two related initialization problems:

1. the default grid's hidden state is not reflected in the initial panel model / control state
2. the first visibility interaction applies the UI's next state from stale initial data instead of
   toggling from the grid's actual runtime visibility

The fix should preserve the intended hidden-by-default behavior and initialize the UI from that
same authoritative visibility state.

## Related

- [Grid panel does not refresh after grid tool placement](./grid-panel-does-not-refresh-after-grid-tool-placement.md)
- [Grid panel does not show grid selection visually](./grid-panel-does-not-show-grid-selection-visually.md)
