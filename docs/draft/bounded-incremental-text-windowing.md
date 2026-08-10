# Bounded incremental text windowing

Status: draft; layout, timing, and authoring semantics remain open.

Related:

- [Incremental text reveal](incremental-text-reveal.md)
- [Manual animation keyframe stepping and XR slide controls](../task/manual-animation-keyframe-stepping-and-xr-slide-controls.md)

## Motivation

Keep incrementally revealed text inside a small, stable reading region. Once the region contains as
many words or lines as it can comfortably show, clear or advance it and begin revealing the next
chunk from the start of that region.

Primary use cases are:

- subtitles near the bottom safe area of a vertical phone video;
- karaoke- or streamer-style captions;
- dialogue boxes with a fixed number of lines;
- speech bubbles with bounded geometry;
- compact XR presentation captions that should not grow indefinitely.

This is distinct from incremental reveal:

- reveal decides when tokens become visible;
- windowing decides which subset of the revealed/source tokens occupies the bounded region.

They should compose, but neither feature should require the other.

## Candidate presentation modes

### Paged replacement

Fill one page, optionally hold it, clear the region, then reveal the next page from its beginning.
This matches dialogue boxes and the requested phone-caption behavior.

### Rolling window

Keep the newest N words or lines visible and evict old content continuously. This resembles live
captions but can be visually busy.

### Caption cards

Pre-segment authored chunks and replace the entire card at explicit timestamps. This gives video
creators exact control but does not automatically adapt to font or viewport changes.

### Highlight window

Keep a bounded phrase visible, with the current word highlighted rather than gradually appending
characters. This is common in short-form social video and may deserve a sibling presentation mode.

No default mode is selected yet.

## What defines the bound?

Possible limits include:

- maximum words;
- maximum grapheme clusters;
- maximum measured lines;
- maximum local width and height;
- maximum duration on screen;
- authored chunk boundaries;
- a combination such as two lines and no more than eight words.

A fixed word count is predictable but ignores word length. Measured geometry adapts better but
depends on font, font size, shaping, language, and target aspect ratio. An implementation intended
for phone-safe captions probably needs measured width/height as its authoritative bound and word
count only as an optional cap.

## Vertical short-form calibration

A future calibration example should render a 9:16 composition or safe-area overlay and let an
author tune:

- output resolution and crop assumptions;
- bottom, left, and right safe margins;
- subtitle region width and maximum lines;
- font size and line spacing;
- words per page or measured page capacity;
- reveal cadence and page hold duration;
- outline/shadow/background treatment;
- placement relative to the VTuber face and hands;
- whether the window appears in the companion view, headset, mirror, or selected combinations.

The current `vtuber-slidedeck` example is intentionally v1 and does not try to calibrate a phone
crop. It provides five complete text states near the avatar so their readability and control flow
can be reviewed first.

## Proposed state machine

A possible paged model is:

```text
Idle
  -> RevealingPage(page_index, visible_tokens)
  -> HoldingCompletedPage(page_index)
  -> ClearingOrTransitioning
  -> RevealingPage(page_index + 1, 0)
  -> Complete
```

Manual input may:

- complete the current page immediately;
- advance from a completed page;
- go to the previous page;
- skip the entire caption;
- replace the caption with a new source generation.

The same button should not accidentally both complete a partial page and advance to the next slide
unless that behavior is explicitly selected.

## Page segmentation

Automatic pagination must know the final font metrics and bounded region. Candidate algorithms:

1. Greedily add tokens until the next token exceeds the line/height limit.
2. Backtrack to preferred punctuation or phrase boundaries.
3. Avoid one-word orphan pages where possible.
4. Respect explicit hard page breaks authored in the source.
5. Cache segmentation by text, style, font, and region dimensions.

Dynamic resizing raises a policy question: should already visible text repaginate immediately,
finish its current page, or retain the segmentation chosen when the reveal began?

## Clearing and transitions

The region could change pages by:

- immediate clear;
- short opacity fade;
- vertical slide;
- old-page fade while new tokens begin;
- keeping prior words dimmed behind the current phrase.

These transitions should compose with the engine's general transition machinery rather than being
hard-coded into text layout. Reduced-motion settings need an immediate alternative.

## Ownership and composition

A likely conceptual split is:

```text
Text source / transcript
        |
window or page segmenter
        |
current page tokens
        |
incremental reveal controller
        |
Text renderer and bounded layout region
```

This makes pre-authored pages, automatically measured pages, and live transcripts share the same
reveal layer. It also allows complete pages with no incremental animation.

## Open questions

1. Is the primary abstraction a subtitle window, a generic text pager, or a layout overflow mode?
2. Are pages computed from word counts, measured geometry, authored breaks, or a hybrid?
3. Which layer owns line breaking: text shaping/layout or the window controller?
4. Should automatic segmentation remain fixed for a reveal generation after it starts?
5. How do font/style changes invalidate cached pagination?
6. What are the default safe margins for 9:16 output, and should they be normalized or pixel based?
7. Should page advancement be time-driven, completion-event-driven, manual, or configurable?
8. Does one button complete the current reveal before it advances the page?
9. How does previous-page navigation interact with a partially revealed current page?
10. Should punctuation and semantic phrase boundaries influence page breaks?
11. How are live speech-recognition corrections handled without distracting repagination?
12. Do hidden/previous pages remain in component topology for transitions or get recycled?
13. How do mirrors, headset eyes, companion output, and vertical capture choose independent caption
    placement and visibility?
14. Is word highlighting part of this feature or a separate timed-text renderer?

## Prototype evidence wanted

- A 9:16 safe-area calibration scene with representative phone UI obstruction overlays.
- Two-line measured pagination across several font sizes and long/short words.
- Paged, rolling, and pre-authored caption-card comparisons.
- Interaction tests for reveal completion, page advance, previous page, and slide advance.
- English, CJK, emoji, punctuation-heavy, and explicit-line-break samples.
- Resize/style-change behavior while a page is partially revealed.
- Allocation, glyph reuse, and layout-recomputation measurements.

The results should determine whether this becomes a generic text/layout primitive or a specialized
subtitle/dialogue component.
