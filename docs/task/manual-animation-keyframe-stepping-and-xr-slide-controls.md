# Manual animation keyframe stepping and XR slide controls

Status: implemented; awaiting hands-on XR example verification.

Related:

- [VTuber slide-deck XR placement and controls](vtuber-slidedeck-xr-placement-and-controls.md)
- [Morph deformation cache plumbing](morph-deformation-cache-plumbing.md)
- [Incremental text reveal](../draft/incremental-text-reveal.md)
- [Bounded incremental text windowing](../draft/bounded-incremental-text-windowing.md)
- [Animation keyframe interpolation](../spec/animation-keyframe-interpolation.md)
- [MMS signal guide](../how_to/guide/signals.md)
- [OpenXR per-hand input state](openxr-per-hand-input-state.md)

## Purpose

Let an MMS `Animation` act as a manually controlled sequence of keyframe callbacks. Authors should
be able to call `next()` and `previous()` without calling `play()`, making an animation useful as a
slide deck, guided demo, test-state selector, or video-production cue sheet.

The motivating workflow is an XR presenter embodied as a VTuber character. The presenter can move
and pose naturally, press controller buttons at chosen moments, and advance text or demonstration
state independently of the engine clock. A slide keyframe may reposition and rotate a text root,
replace its text, change font size and color, and switch the active state of the demonstrated
feature.

## Original behavior and implemented seam

Implementation note (2026-08-10): the core MMS methods, intent routing, manual cursor, clamping,
playback interaction, and visual-only keyframe execution described below are implemented. The
`vtuber-slidedeck` example binds `ButtonB` to `next()` and `ButtonA` to `previous()` for hands-on
verification. A desktop input fallback and the morph-target-specific deck remain follow-up work.

`AnimationComponent` currently has `Playing`, `Looping`, and `Paused` states. `AnimationSystem`
stores registered keyframes in beat order and evaluates due callbacks from clock progress.

Before this change, the live MMS methods were:

- `play()`, which starts a one-shot animation from the beginning;
- `loop_anim()`, which starts a looping animation from the beginning;
- `pause()`, which stops clock-driven evaluation.

Previously there was no manual keyframe cursor, step intent, or `next()`/`previous()` method.
Those pieces now use the normal signal/intent drain path and the existing visual keyframe
evaluator.

XR input exposes `XrButtonDown` events with controls including `ButtonA`, `ButtonB`, `ButtonX`,
and `ButtonY`. The new animation methods use the normal intent pipeline so XR, desktop buttons,
scripts, and other event sources all receive identical behavior.

## Proposed MMS surface

The smallest authoring surface uses the existing paused constructor:

```mms
let slides = Animation.paused() {
    Keyframe.at(0) {
        slide_text.set_text("Cached skinning")
        slide_text.set_font_size(0.8)
        slide_color.set_rgba(0.3, 0.8, 1.0, 1.0)
        slide_root.update_transform(
            [0.0, 1.8, -1.5],
            [0.0, 0.0, 0.0],
            [1.0, 1.0, 1.0],
        )
    }

    Keyframe.at(1) {
        slide_text.set_text("Morph-capable, all factors zero")
        slide_text.set_font_size(0.65)
        slide_color.set_rgba(1.0, 0.75, 0.25, 1.0)
        morph_probe.all_zero()
    }

    Keyframe.at(2) {
        slide_text.set_text("One identity morph active")
        slide_color.set_rgba(0.4, 1.0, 0.55, 1.0)
        morph_probe.identity_active()
    }
}

slides.next()
slides.previous()
```

`Animation { ... }` values should also expose these methods. Calling a manual step on a playing or
looping animation first pauses clock-driven playback, then performs the requested step. Authors
who want no automatic playback before the first input should use `Animation.paused()` explicitly;
changing the existing default constructor from looping to paused is not part of this task because
it could break existing scenes.

Possible follow-up methods are `first()`, `last()`, `go_to(index)`, and a current-index query. They
are useful for random access but do not block the initial `next()`/`previous()` slice.

## Manual cursor semantics

Maintain a manual cursor per live animation runtime. The cursor identifies a keyframe by
`ComponentId`; its ordinal is derived from the current deterministic keyframe ordering.

Version 1 behavior:

- A newly registered animation's cursor is before the first keyframe.
- `next()` from the initial state selects and executes the first keyframe.
- `next()` selects and executes the following keyframe.
- `previous()` selects and executes the preceding keyframe.
- `previous()` before the first keyframe is a no-op.
- `previous()` on the first keyframe and `next()` on the last keyframe clamp and do not re-fire.
- Manual stepping does not wrap. Explicit wrapping may be added later.
- Each successful step executes exactly one selected keyframe callback once.
- Ordinary visual playback updates the cursor to the most recently fired keyframe. Calling
  `next()` during playback therefore pauses and advances from the state currently on screen rather
  than unexpectedly returning to the first slide.
- Equal-beat keyframes remain distinct manual steps and use deterministic authored/registration
  order as their tie-breaker.
- Adding or removing keyframes refreshes ordering. If the selected keyframe disappears, preserve
  the nearest valid ordinal where possible; otherwise return to the before-first state.

Manual selection is independent of beat spacing. Beats retain their meaning for ordinary playback
and ordering, but stepping from beat `1` to beat `100` still advances exactly one slide.

## Forward and backward execution

Keyframe callbacks are imperative and may perform arbitrary mutations. The animation system cannot
generically reverse a callback.

Therefore, `previous()` means:

1. select the previous keyframe;
2. execute that keyframe's callback again;
3. rely on the callback to describe the complete desired state for that slide.

Presentation and test-deck keyframes must be idempotent and state-complete. A slide should set its
text, transform, color, font size, and demonstration mode explicitly rather than relying on changes
made by the slide visited immediately before it.

This also means a callback such as `counter.increment()` is unsafe for reversible navigation,
while `counter.set(3)` is suitable.

## Execution and intent contract

Add a step intent owned by `AnimationSystem`, for example:

```text
StepAnimation {
    component_id,
    direction: Next | Previous,
}
```

Live MMS method dispatch maps `animation.next()` and `animation.previous()` to this intent. The
intent executes at the normal signal drain point; it must not directly mutate the world from the
event handler.

The selected keyframe should use the existing keyframe callback evaluator in visual/manual mode so
component methods continue to emit their normal intents. Manual stepping should not participate in
audio lookahead. Explicit audio triggered by a future manual-step API needs a separately documented
policy; v1 slide stepping is visual and state-control oriented.

Successful manual stepping pauses clock playback without resetting the manual cursor. Calling
`play()` or `loop_anim()` preserves the existing restart semantics and resets the manual cursor to
before-first; subsequent visual playback advances it as keyframes fire. Calling `pause()` alone
preserves the current cursor.

## XR controller binding

The initial presentation convention is:

- `ButtonB`: next slide;
- `ButtonA`: previous slide.

Use `XrButtonDown`, not button-held or axis-change events, so one physical press requests one step:

```mms
let xr_controls = InputXRGamepad {
    locomotion()
}

on(xr_controls, "XrButtonDown", fn(event) {
    if event.control == "ButtonB" {
        slides.next()
    } else if event.control == "ButtonA" {
        slides.previous()
    }
})
```

Locomotion and avatar hand/controller systems remain active while slides are stepped. The deck is
an independent presentation channel and must not take ownership of the avatar pose or XR clock.

Provide a desktop fallback in the example through clickable previous/next controls or keyboard
events so the deck can be tested without a headset. The core animation API must not depend on XR.

## Slide mutation surface

The first demonstration should cover live component methods already supported by keyframe blocks:

- text content through `set_text(...)`;
- text size through `set_font_size(...)`;
- color through the live color mutation method;
- translation, rotation, and scale through `update_transform(...)`;
- feature-specific state through ordinary component methods or intents.

Selecting a different font family or font asset is not currently part of `TextComponent`'s live
mutation surface. If the example requires font-family changes, add and test a dedicated text/font
mutation API rather than overloading `set_font_size` or rebuilding the text subtree in the
animation system.

## Morph-target example integration

The planned `vtuber-morph-targets` validation example uses this API as an operator-controlled test
deck. Its `.rs` wrapper and `.mms` scene are specified in
[Morph deformation cache plumbing](morph-deformation-cache-plumbing.md).

Suggested slides are:

1. skin-only baseline;
2. morph-capable mesh with all morph blend factors zero;
3. one active zero-delta identity target;
4. change one identity target factor once;
5. unchanged factors while the skeleton continues moving;
6. summary counters and expected behavior.

The animation-stepping task is useful for operating and recording this example, but it must not
block automated morph cache tests or command-line performance presets.

## Tests

- `next()` on a paused animation executes the first keyframe without `play()`.
- Repeated `next()` visits every keyframe once and clamps at the end.
- `previous()` re-applies the prior keyframe and clamps at the beginning.
- Equal-beat keyframes have deterministic stepping order.
- A step on a playing or looping animation pauses it before executing one keyframe.
- A step during playback advances from the most recently fired visual keyframe.
- `play()` and `loop_anim()` retain restart behavior and reset the manual cursor.
- `pause()` preserves the cursor.
- Keyframe removal and insertion do not leave a dangling cursor.
- Step requests flow through intents and execute at a drain point.
- A stepped callback can update text, font size, color, transform, and feature state together.
- `ButtonB` and `ButtonA` `XrButtonDown` events cause one next/previous request respectively.
- Held buttons do not repeatedly advance slides without new down events.
- Avatar locomotion, tracking, mirrors, and ordinary animation remain active while the deck is
  manually stepped.

## Completion criteria

- MMS animations expose working `next()` and `previous()` live methods without requiring
  `play()`.
- Manual cursor, clamping, playback interaction, topology changes, and callback execution are
  deterministic and tested.
- Backward navigation is documented as state reapplication, not automatic reversal.
- XR A/B controls can navigate a state-complete text/demo slide deck while the user remains in
  control of a tracked VTuber avatar.
- A desktop fallback demonstrates that the stepping API is input-source independent.
- The morph-target validation example uses the deck when available without making automated morph
  tests depend on it.
