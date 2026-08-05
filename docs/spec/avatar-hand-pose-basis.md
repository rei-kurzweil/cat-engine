# Avatar hand pose basis (superseded)

Date: 2026-08-04

Status: superseded by the generic joint-basis and rest-attachment contracts for Mittens Engine
0.8.

The former avatar-hand helper mixed XR landmark selection, imported-joint basis correction, and
fingertip pointer placement. That compatibility path has been removed.

Use:

- [`JointRetargetBasis`](joint-retarget-basis-component.md) to define the canonical two-axis rest
  frame for an imported target joint;
- [`JointBasisRetargetingSystem`](joint-basis-retargeting-system.md) to retain and share the basis
  between pose consumers; and
- [`RestAttachment`](rest-attachment-component.md) to define an imported target's rest-space
  attachment offset independently from orientation.

XR is now only a pose source. Future `HumanoidBoneMap` support will resolve semantic slots into the
same generic retained definitions rather than restoring an XR-specific recipe.
