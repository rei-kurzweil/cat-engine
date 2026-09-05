# Default grid visibility UI is out of sync on editor startup

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
