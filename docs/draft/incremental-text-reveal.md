# Incremental text reveal

Status: draft; behavior and API are intentionally unsettled.

Related:

- [Bounded incremental text windowing](bounded-incremental-text-windowing.md)
- [Manual animation keyframe stepping and XR slide controls](../task/manual-animation-keyframe-stepping-and-xr-slide-controls.md)
- [Animation keyframe interpolation](../spec/animation-keyframe-interpolation.md)

## Motivation

Support text that appears incrementally instead of replacing the complete string in one frame.
The same primitive should cover:

- word-at-a-time short-form-video captions;
- character-at-a-time visual-novel and RPG dialogue;
- terminal/typewriter effects;
- emphasized pauses around punctuation;
- manually advanced presentation text.

This is adjacent to `Text.set_text(...)`, but it is not merely repeated substring mutation. A
robust implementation must define Unicode segmentation, layout stability, cancellation, timing,
skip behavior, and interaction with bounded subtitle pages.

## Conceptual state

An incremental reveal has at least:

```text
complete source text
segmentation mode: grapheme | word | authored token
revealed token count
cadence or per-token timing
playback state
layout policy
completion/cancellation generation
```

The displayed prefix is derived state. The full source remains available so layout can optionally
reserve final geometry before the first token appears.

## Candidate authoring surfaces

Component-oriented:

```mms
TextReveal.words() {
    interval_ms(90)
    punctuation_pause_ms(180)
    target(subtitle_text)
}
```

Live text method:

```mms
subtitle_text.reveal("you can cache this once", {
    unit = "word"
    interval_ms = 90
})
```

Animation/keyframe integration:

```mms
Keyframe.at(2) {
    subtitle_text.reveal_words("chat asked for another mirror")
}
```

No API is selected yet. A component makes state inspectable and composable; a text method is more
convenient; an animation-only feature would be too narrow.

## Segmentation

Character mode should mean Unicode grapheme clusters, not bytes, Unicode scalar values, or UTF-8
code units. Emoji sequences, combining accents, and variation selectors must not be split into
broken intermediate glyph strings.

Word mode needs a policy for:

- whitespace preservation;
- punctuation attached to the preceding or following word;
- explicit line breaks;
- emoji and CJK text without ASCII spaces;
- authored emphasis spans or future rich-text markup.

An authored-token mode may eventually accept explicit chunks for exact subtitle timing.

## Timing choices

Possible clocks include:

- real elapsed milliseconds, natural for dialogue and video captions;
- animation beats, natural for music-synchronized sequences;
- explicit timestamps per token, natural for transcription and editing;
- manual advance, natural for presentation and accessibility.

The first implementation should not hide conversion between beats and seconds. The chosen clock
must be explicit, deterministic, and compatible with pause/resume.

Punctuation delays are useful but should be policy rather than hard-coded English assumptions.
Question marks, commas, ellipses, line breaks, and authored pause markers may each want different
durations.

## Layout stability

Two valid presentation modes conflict:

1. Incremental layout: only revealed text participates in measurement. This feels like typing but
   can move centered or wrapped text every time a token appears.
2. Reserved layout: measure the complete source immediately while rendering only its revealed
   prefix. This avoids jitter and is usually preferable for subtitles and speech bubbles.

The text/layout boundary needs a representation for hidden-but-measured glyphs if reserved layout
is supported efficiently. Rebuilding the complete glyph subtree for every character would be a
poor default.

## Mutation and cancellation

Starting a new reveal on a text component with one already active needs an explicit policy:

- replace and restart;
- finish the old reveal immediately, then start;
- queue;
- reject overlapping requests.

`set_text(...)` during a reveal must likewise define whether it cancels the reveal or changes its
source. A generation token is likely useful so stale scheduled callbacks cannot mutate newer text.

Common controls include:

- pause/resume;
- skip to complete;
- cancel and clear;
- reveal one additional token;
- query whether the reveal is complete;
- emit token/page/completion events.

## Performance direction

The implementation should avoid reconstructing unrelated component topology for every token.
Candidate approaches include:

- retain glyphs for the complete source and change per-glyph visibility;
- append only newly visible glyphs while retaining prior glyphs;
- update a compact visibility/count value consumed by text rendering;
- cache shaping and layout for the complete source, then reveal from that cache.

The correct choice depends on the eventual font/shaping path. This draft does not select one.

## Interaction with manual slide decks

A stepped slide should be able to start a reveal while leaving the slide itself selected. Going to
the previous or next slide cancels any reveal generation owned by the old slide. Re-entering a
slide should deterministically restart or restore its reveal according to an authored policy.

For recorded short-form video, a useful flow is:

1. press B to select the next slide;
2. the slide installs its complete caption source;
3. words reveal on a millisecond cadence;
4. press B again whenever the presenter is ready for the next authored state.

## Open questions

1. Is reveal state a child component, a live `Text` method, or both?
2. Should the default character unit be grapheme clusters?
3. Is word segmentation provided by Unicode rules, whitespace rules, or authored tokens?
4. Should real-time milliseconds or animation beats be the default clock?
5. How are punctuation pauses authored and localized?
6. Should complete text reserve its final layout by default?
7. Can hidden glyphs remain measured without becoming raycastable or visible?
8. What happens when `set_text(...)` arrives during a reveal?
9. Should a second reveal replace, queue behind, or complete the first?
10. Which events are needed for audio, lip-sync, sound effects, and subtitle paging?
11. How should skipping behave when accessibility settings prefer reduced motion?
12. Can a reveal survive text-style changes without restarting shaping/layout work?

## Prototype evidence wanted

- One grapheme-at-a-time visual-novel dialogue sample.
- One word-at-a-time short-form-video caption sample.
- Comparison of incremental versus reserved layout for centered and wrapped text.
- Cancellation and replacement while a reveal is mid-flight.
- Unicode fixtures containing combining marks, emoji sequences, CJK, punctuation, and newlines.
- CPU allocation and component-tree churn measurements for a long paragraph.

The prototype should inform a later committed task/spec rather than silently establishing permanent
API semantics.
