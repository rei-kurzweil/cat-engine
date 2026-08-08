# Task: VTuber example mirror and editor-UI spacing

Date: 2026-08-08

Status: todo

## Goal

Bring the scene mirror forward in both VTuber editor examples so its reflective surface sits about
`0.4` to `0.5` world units behind the editor UI plane.

Affected scenes:

- `examples/vtuber-mirror-example.mms`
- `examples/vtuber-editor-example.mms`

## Current state

The mirror geometry is currently authored as ordinary scene transforms inside an `ED` subtree:

- `vtuber-mirror-example`: mirror near `z = -4.5`
- `vtuber-editor-example`: mirror near `z = -3.95`, with a backing/frame near `z = -4.08`

The editor UI is runtime-generated, so the desired change should be measured relative to its
settled world-space plane rather than implemented by blindly copying one absolute Z value between
the examples.

## Work

1. Run each example in its intended XR/editor configuration.
2. Record the settled editor UI plane, its facing direction, and the mirror reflective plane.
3. Move the complete mirror assembly along the UI normal until the reflective surface is roughly
   `0.4`–`0.5` units behind the UI.
4. Preserve the mirror frame/backing offset in `vtuber-editor-example`.
5. Check that the mirror does not intersect panels, avatar geometry, temple geometry, or controller
   interaction space.
6. Confirm the mirror remains useful from the default headset pose and after ordinary locomotion.

## Acceptance criteria

- [ ] Both examples place the reflective plane `0.4`–`0.5` world units behind the editor UI plane.
- [ ] “Behind” is measured along the UI's resolved normal, not assumed to mean global `-Z`.
- [ ] Mirror, frame, and backing remain visually aligned.
- [ ] Editor panels do not z-fight with or clip into the mirror.
- [ ] The default XR pose shows the UI and its reflection at a useful scale.
- [ ] The existing mirror render-view and selection behavior still works.
- [ ] The final measured UI and mirror poses are recorded in this tracker or a verification note.

## Non-goals

- Changing mirror rendering, quality, or camera semantics.
- Reworking editor-panel placement generally.
- Making the mirror dynamically follow the UI in this first example-specific adjustment.

