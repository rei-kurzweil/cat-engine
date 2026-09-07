# Secondary motion rest-pose return and Bisket bust tuning

Date: 2026-09-06

Status: proposed task tracker

## Problem

The Bisket secondary-motion preset is broadly useful, but its two bust chains settle downward and
do not convincingly return to their relaxed authored pose. More generally, the current examples
demonstrate inertia, drag, gravity, and collision, but do not prove that a displaced spring chain
can converge back to an independently defined primary/rest pose.

The visible symptom is easy to describe as excessive gravity, but this task must distinguish a
preset-tuning issue from a solver-target issue. Setting bust gravity to zero is a useful experiment;
it is not sufficient proof that the solver has stable restorative behavior.

## Current evidence

The Bisket preset currently configures each bust chain as:

```mms
SpringBone.from_root(chain[0])
    .stiffness(4.0)
    .drag_force(0.60)
    .gravity(0.35, [0, -1, 0])
```

Nonzero world-space gravity should produce an equilibrium displaced from the rest pose. If the bust
should naturally hold its authored shape, its preset probably needs zero or nearly zero gravity and
enough damping to settle without prolonged oscillation.

There is also a deeper concern in `SecondaryMotionSystem::simulate_step`:

- binding stores immutable imported rest rotations and rest directions;
- applying the result composes a correction with each imported rest rotation;
- the per-step stiffness target, however, is reconstructed from current joint world positions;
- those positions can already include the previous frame's secondary-motion output when no
  animation or other primary-pose producer rewrites the chain every frame.

This creates a feedback risk: the spring can treat its own last output as the next primary target.
A one-shot relaxed-pose overlay makes the distinction especially important. "Rest pose" may mean
the imported GLTF bind pose, while the desired restorative target may actually be the current
authored/animated primary pose after the relaxed overlay.

## Goal

Make rest/primary-pose attraction an explicit, testable secondary-motion behavior and tune the
Bisket bust so it returns to its relaxed shape after movement instead of continually yielding to
gravity.

The intended invariant is:

> With gravity and collisions disabled, a displaced chain with positive stiffness and drag
> converges toward the pose supplied by the primary animation/pose stage, and its residual motion
> converges toward zero.

With gravity enabled, the chain may settle away from that pose. That displacement must be an
intentional force balance rather than accumulation caused by feeding prior secondary output back
into the stiffness target.

## Terminology and ownership

- **Imported rest pose**: immutable local TRS captured from the GLTF at import time.
- **Primary pose**: local/world joint transforms produced this frame by rest pose, pose overlays,
  animation, IK, or avatar control before secondary motion.
- **Secondary output**: the correction written by the spring solver after the primary pose.
- **Restoration target**: the primary-pose tail direction that stiffness attempts to recover.

The restoration target should normally be the current primary pose, not necessarily the immutable
GLTF bind pose. Immutable rest data remains the fallback when no primary producer is active and is
still required for stable chain topology and local correction bases.

## Investigation plan

### Phase 1: establish the actual failure mode

- [ ] Add a deterministic one-segment solver test with zero gravity, no colliders, positive
      stiffness, and positive drag. Displace the simulated tail, run fixed steps, and assert
      convergence in both position error and velocity.
- [ ] Add the equivalent two-segment test because the Bisket bust has two bones and the terminal
      segment uses a virtual endpoint.
- [ ] Keep the primary pose constant during those tests without rewriting it from the solver's own
      output. Confirm whether the current implementation converges or moves its target.
- [ ] Add a test where an upstream primary pose changes, stops, and remains stable. The chain should
      follow with lag and then settle onto the new target.
- [ ] Add a nonzero-gravity test proving that a stable displaced equilibrium is distinct from
      unbounded or self-referential drift.
- [ ] Run the same tests with collisions enabled but nonintersecting to ensure merely configuring
      colliders does not alter convergence.
- [ ] Capture per-step target direction, simulated direction, angular error, velocity, stiffness,
      drag, and gravity for one Bisket bust chain behind a narrow debug flag.

### Phase 2: establish an independent primary-pose input

- [ ] Audit the frame order between pose/animation, avatar control, IK, transform propagation,
      secondary simulation, secondary writes, and skinning.
- [ ] Stop deriving the restoration target from transforms that may contain the previous
      secondary output.
- [ ] Choose and document one primary-input strategy:
  - snapshot primary transforms after primary pose/IK propagation and before secondary writes;
  - retain the previous secondary correction and remove it from the next frame's local transform;
    or
  - represent primary and secondary transforms as separate pipeline values and compose them only
    for final world propagation.
- [ ] Prefer an explicit pre-secondary primary snapshot or separate pipeline value. Removing the
      previous correction is a narrower compatibility approach but is more sensitive to ordering
      and concurrent pose writers.
- [ ] Compute stiffness from the independent primary/restoration direction while continuing to
      enforce segment length and collider constraints on simulated positions.
- [ ] Define terminal virtual-endpoint restoration from authored local rest direction rather than
      extrapolating solely from already-modified current positions.
- [ ] Ensure the correction is still composed in the correct local basis for nested chain joints.
- [ ] Reset or rebase simulation state cleanly when the primary pose changes discontinuously,
      chains are rebound, or secondary motion is toggled.

### Phase 3: tune and demonstrate the Bisket bust

- [ ] First set the Bisket bust gravity to `0.0` and observe whether the corrected solver returns to
      the relaxed pose without permanent sag.
- [ ] Tune bust stiffness and drag only after the convergence invariant passes. Keep hair and tail
      settings unchanged.
- [ ] If a small gravitational bias is aesthetically useful, add it back deliberately and document
      the expected equilibrium offset.
- [ ] Add a focused Bisket demonstration with a repeatable displacement or root-motion pulse,
      followed by a stationary interval long enough to see the bust settle.
- [ ] Show the restoration target and simulated chain through the existing secondary-motion
      visualization/snapshot path when debug visualization is enabled.
- [ ] Verify both left and right bust chains behave symmetrically in the relaxed A-pose.
- [ ] Recheck the two models in `examples/shading-models.mms` so secondary-motion differences do
      not obscure the shading-model comparison.

## Proposed first experiment

Change only the two bust-chain preset entries from:

```mms
.gravity(0.35, [0, -1, 0])
```

to:

```mms
.gravity(0.0, [0, -1, 0])
```

This is deliberately an experiment, not the final fix. Outcomes distinguish likely causes:

- If the bust returns cleanly, the solver already has adequate restoration and the main task is
  preset tuning plus regression coverage.
- If it remains displaced or its target follows the simulated output, the solver needs an
  independent primary-pose target before preset tuning is meaningful.
- If it returns to the imported bind pose but not the relaxed overlay, the solver must distinguish
  imported rest from the current primary pose.

## Acceptance criteria

- A zero-gravity spring displaced from a stationary primary pose converges to a documented angular
  and positional tolerance within a bounded time.
- Residual velocity also converges; the test must not pass merely because position crosses the
  target during an undamped oscillation.
- A two-joint chain with a virtual endpoint satisfies the same convergence guarantee.
- Repeated secondary-motion ticks do not move the primary/restoration target when no upstream
  system changes it.
- A changed primary pose becomes the new restoration target without snapping during ordinary
  continuous motion.
- Nonzero gravity produces a stable, explainable equilibrium offset.
- The Bisket bust returns close to its relaxed A-pose after a repeatable movement and stationary
  settling period, without visible long-term downward drift.
- Hair and tail behavior do not regress from the bust-specific preset change.
- Fixed-step determinism and the existing steady-state no-discovery/no-allocation expectations are
  preserved.

## Tests and evidence

- Unit tests for one-segment and two-segment zero-gravity convergence.
- Unit test for primary-pose retargeting followed by convergence.
- Unit test for bounded nonzero-gravity equilibrium.
- Regression test for terminal virtual-endpoint rest direction.
- Bisket integration test resolving both bust chains from the real GLTF.
- Before/after capture using a fixed camera, deterministic movement pulse, and fixed settling time.
- Optional convergence trace recording angular error and endpoint velocity over time.

## Non-goals

- Retuning Bisket hair or tail unless the solver correction exposes a shared regression.
- Adding soft-body volume preservation or anatomical simulation.
- Replacing the fixed-step integrator solely for this task.
- Building the general secondary-motion editor panel.
- Treating gravity as inherently incorrect for every spring chain.

## Questions to settle

1. Is the desired Bisket bust target the imported bind pose or the relaxed-pose overlay after it is
   applied?
2. Which system owns the authoritative pre-secondary primary transform snapshot?
3. Should stiffness remain the current unitless VRM-style coefficient, or should future work expose
   a time-normalized spring frequency/damping model?
4. Does a primary-pose discontinuity reset simulated velocity, rebase endpoints while preserving
   velocity, or use a configurable policy?
5. Is zero gravity the correct shipped bust default, or should a small bias remain after solver
   restoration is proven?
