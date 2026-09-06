# Task: release zones for sockets and vehicle mounting

## Status and outcome

Planned. Part of [grabbing, poses, and release zones](epic/grabbing-poses-and-release-zones.md).
The broom asset will be supplied later; authored proxy geometry can validate
the attachment contract first.

A held component tree can become eligible for an authored spatial zone.
Releasing inside an eligible zone commits a snap/attachment action. Proximity
while holding must not itself attach or mount.

## Shared zone contract

Separate the eligibility volume from the destination transform. A zone declares
accepted trees or capabilities, its volume and coordinate frame, an authored
snap anchor, and an action. Exact component/API names remain open.

Define which point or bounds of the held tree is tested, overlap versus
containment, exclusions, and any entry/exit hysteresis. Reevaluate eligibility
at release; preview state is not sufficient authority. Choose at most one
winner using an explicit deterministic priority/distance/tie-break policy.
A zone may reject incompatible or occupied attachments.

Release handling must choose ordinary drop or a zone action as one coordinated
handoff. Existing grab release restores the original parent while preserving
world pose; it must not race a zone's new attachment. Invalid or removed zones
fall back to a normal release. Specify explicit detach/regrab and cleanup when
an attached tree, anchor, or owner is removed.

## Socket action: prop to avatar

A mouth zone accepts a suitable prop, such as a lollipop or marijuana cigarette.
On release, the prop's authored grip/contact transform aligns to a mouth socket
with an authored position and orientation. The prop thereafter follows that
socket. Snapping the prop's arbitrary root origin is not sufficient.

The attachment may use tree reparenting or an effective transform-parent basis;
select the mechanism after reviewing existing transform contracts. The required
behavior is explicit and persistent following, regardless of that choice.

## Mount action: avatar to broom

While held, the broom follows the grabbing pointer's nearest ancestor transform.
An avatar-relative zone near the legs accepts it, with the torso excluded.
Releasing there mounts; merely moving through the zone does not.

The action reverses which tree follows which:

1. Validate the broom, rider, riding anchor, eligibility, and physics handoff.
2. Remove the broom's hand attachment and establish its independent world/physics
   basis, preserving its world pose before the authored mount alignment.
3. Attach the avatar's designated movement/root transform to the broom's riding
   anchor and apply the authored riding alignment/pose.
4. Give broom physics authority over vehicle motion; coordinate avatar locomotion
   and collision ownership so two drivers do not compete.

Never attach the avatar beneath a broom that is still beneath the avatar's hand.
Check both structural and effective transform-basis cycles. Commit the operation
atomically from the consumer's perspective; failure must leave a valid ordinary
release, not a partially mounted tree or a lingering hand attachment.

Define dismount/regrab behavior, restoration of avatar movement/collision
ownership, and cleanup if the broom disappears before enabling the feature.
Vehicle controls, flight forces, velocity inheritance, and collision details
need a concrete policy during implementation; this ticket does not presume
that an existing physics path already provides them.

## Acceptance criteria

- Releasing a compatible prop in the mouth zone aligns its authored contact to
  the socket and follows subsequent avatar/head motion; regrabbing detaches it.
- Entering either zone while holding does not commit. Releasing outside or in
  an incompatible zone performs ordinary release.
- The leg zone mounts the broom on release; torso-only proximity does not.
- Mounted avatar motion follows the physics-driven broom without a transform
  cycle, double driving, or a transient return to the old grab parent.
- Overlapping zones select one deterministic action; removed/occupied targets
  and failed handoffs leave valid attachment and grab state.
- Dismount and owner removal restore a usable avatar and consistent physics state.

## Related work

- [Effective transform-parent basis resolution](../draft/effective-transform-parent-basis-resolution.md)
- [Transform pipeline](../spec/transform-pipeline.md)
- [Grab pose transitions](grab-animation-and-pose-transitions.md)
