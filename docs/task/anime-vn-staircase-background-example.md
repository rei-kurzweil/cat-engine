# Task: Anime VN staircase background MMS examples

Date: 2026-08-29

Status: proposed

## Goal

Add two runnable MMS versions of the same first-person, anime visual-novel
background scene:

- a desktop-camera version;
- an XR-camera version.

Both place the canonical Bisket avatar, including its existing secondary
motion, spring colliders, and imported morph targets, in a landscaped
staircase setting.  The examples are an integration target for reusable MMS
environment assets rather than a one-off collection of scene geometry.

The terrain beside and beneath the stairs requires the follow-up
[implicit 3D surfaces and CSG task](implicit-3d-surfaces-and-csg.md).  Until
that work exists, use explicit ground planes and a deliberately temporary
terrain stand-in; do not make the example depend on an invented API.

## Deliverables

1. `examples/anime-vn-staircase-desktop.mms`
2. `examples/anime-vn-staircase-xr.mms`
3. `assets/components/railing.mms`, exporting a reusable `railing(...)`
   factory rooted in `CombineMesh`.
4. `assets/components/minimal_outdoor_light.mms`, exporting a reusable,
   compact street-light factory with a downward-facing `SpotLight`.
5. A focused update to the older street-light code in
   `examples/gravity-fields.rs`, replacing its point-light-only treatment
   with the same downward-lighting intent where that example's API permits.
   It is not necessary to port that Rust example to MMS as part of this task.

## Shared composition

### Viewer and subject

The scene is first person.  Bisket is the visible avatar body associated with
the viewer, not a separate static character placed in the shot.  Reuse the
canonical pieces already used by the Bisket examples:

- `assets/models/bisket.glb`;
- `assets/components/secondary_motion/bisket.mms`;
- `assets/components/colliders/bisket.mms`; and
- the Bisket humanoid-map / avatar-control topology appropriate to each
  viewer type.

The desktop version should follow the established `Input` + `AVC` + `C3D`
first-person pattern.  The XR version should follow the established
`InputXR` + `AVC` + `CXR` pattern, retaining the tracked head and hands.

The two files should share imported environment assets and the same authored
scene dimensions.  Duplication of the small camera/AVC setup is acceptable
until MMS has a clean scene-configuration module pattern.

### Stair orientation and dimensions

From the camera's forward direction, the staircase descends at a yaw of
45 degrees to the left.  Author a single `scene_yaw` constant and derive the
stair/path placement from it; do not encode an unrelated rotation per object.

The clear stair width is **8 world units**.  It has three identical descending
flights separated by two square landings:

| segment | count / size |
| --- | --- |
| flight 1 | 18 steps, 8 units wide |
| landing 1 | 8 x 8 units |
| flight 2 | 18 steps, 8 units wide |
| landing 2 | 8 x 8 units |
| flight 3 | 18 steps, 8 units wide |

Choose and document shared `step_run` and `step_rise` constants in the scene;
each flight must derive both its pitch and total drop from those constants.
The initial implementation may use repeated cube steps.  It should also add a
sloped structural underbody so the stairs read as a designed outdoor feature,
not floating treads.

At the bottom, place a reddish-brown brick-like path.  It runs straight forward
from the final flight for a short stem, then forms a left/right T split.  Each
branch is approximately three times the stem length.  A slightly varied
reddish-brown material is sufficient for the first version; actual brick
texture mapping is a later visual enhancement, not a blocker.

### Ground and hills

Use flat/rotated planes for the bottom ground, upper ground, and any simple
joins that can be represented without curvature.  The intended final setting
has rolling hills formed from overlapping implicit spheres, constrained and
trimmed around the stairs and path using CSG.  That exact terrain is blocked
on [implicit 3D surfaces and CSG](implicit-3d-surfaces-and-csg.md).

Keep the terrain boundary and the temporary substitute isolated in a helper so
the scene can swap to the final terrain without moving the staircase.

## Reusable railing asset

`railing(...)` is a generic factory, not a staircase-specific helper.  Its
returned root **must** be `CombineMesh { ... }`; every post, rail, and
diagonal member is authored below that root so the fixture bakes as one visual.

### Proposed authoring surface

```mms
import { railing } from "../assets/components/railing.mms"

railing({
    length = flight_length
    inclination = -flight_pitch
    pole_count = 7
    start_kink_distance = 0.55
    end_kink_distance = 0.55
    height = 1.05
    rail_thickness = 0.09
    post_thickness = 0.11
    side = "left"
})
```

Exact MMS spelling may be adjusted to the language's supported parameter
conventions, but these semantics are required:

- `length`: distance along the railing's local longitudinal axis;
- `inclination`: signed pitch of the sloped middle span; zero yields a flat
  railing;
- `pole_count`: number of vertical poles, including the end poles; spacing is
  derived from `length / (pole_count - 1)`;
- `start_kink_distance` and `end_kink_distance`: flat horizontal portions at
  the start/end before or after the sloped span; when pitched, each must extend
  slightly past its adjacent end pole so the rail has a deliberate kink rather
  than beginning its slope exactly at the post;
- `height`, `rail_thickness`, and `post_thickness`: sensible geometric
  controls; and
- `side`: optional semantic metadata or placement aid if it proves useful;
  mirrored transforms remain valid.

Reject invalid geometry at the factory boundary: `pole_count < 2`, negative
length, negative kink distances, or kink distances whose sum exceeds the
length.  Clarify whether poles sit on the local support line or vertically in
world/up space; for the staircase use case, they must remain vertical while
the rail follows the incline.

### Stair usage

Every stair flight gets one railing on each side.  Each side comprises:

1. the top sloped handrail following that flight; and
2. a second, parallel sloped member, locally rotated around Z from the first
   to form the long descending corner/triangular side profile requested for
   the staircase.

Every landing also gets two long flat side rail pieces, one on each side.  Set
them slightly above their support surface—roughly one-half rail height or one
rail thickness—so they read as separate fabricated rails rather than flush
geometry.  Landing rails have zero inclination and no diagonal side member
unless a later composition pass establishes that it improves the silhouette.

## Reusable minimal outdoor light

Create a simple, conservative outdoor street-light asset inspired by
`tripod_light`, without the tripod or a long projecting arm.  It should have:

- a tall, narrow vertical pole;
- a compact cap/housing close to the pole top;
- a small downward-facing diffuser with restrained emissive material; and
- a downward-facing `SpotLight` aligned with that diffuser.

The factory should accept a name, position or placement transform, a light
component/configuration, and compact dimensions/colors.  Keep the horizontal
overhang short.  Use a warm, low-drama spot-light default suitable for an
anime VN background; the scene should rely on modest fill/directional light as
well, rather than using a single intense pool of light.

Place a small number along the landings/path only after the base scene reads
well unlit.  Their purpose is depth and path guidance, not visual clutter.

## Implementation order

1. Build and test the railing asset in a minimal isolated MMS scene, including
   flat and inclined/kinked cases.
2. Build and test the minimal outdoor light asset and its spotlight aim.
3. Block out the shared stair/path geometry with explicit plane ground.
4. Add the desktop first-person Bisket variant with secondary motion,
   colliders, and morph-target-capable model loading.
5. Add the XR variant by adapting only the viewer/avatar-control topology.
6. Replace the terrain stand-in once the linked implicit-surface task has an
   accepted MVP.
7. Update the legacy gravity-fields street-light behavior as a contained
   follow-up and verify it remains visually functional.

## Acceptance criteria

- Both scene files run, start in first person, and present equivalent world
  geometry and scale.
- The stairs are visibly 8 units wide, have three 18-step flights and two
  8-by-8 landings, and descend 45 degrees left of forward.
- The railings span every flight and landing; flight rails have vertical posts,
  a pitched/kinked upper rail, and the second descending corner member.
- The railing asset is reusable, parameterized, and rooted in `CombineMesh`.
- Bisket loads through the normal avatar stack with Bisket secondary motion
  and colliders enabled; it does not use a simplified static substitute.
- The bottom path has a forward stem and left/right branches roughly three
  times as long as that stem, rendered reddish brown.
- Outdoor lights use spotlights aimed downward; no long horizontal arm is
  needed for the initial asset.
- The temporary terrain seam is explicit, and the final implicit/CSG terrain
  remains tracked by the linked task rather than silently omitted.
