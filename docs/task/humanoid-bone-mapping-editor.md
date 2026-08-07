# Future task: humanoid bone-mapping editor

Date: 2026-08-06

Status: deferred beyond 0.7.1

Build an editor panel over the retained `HumanoidBoneMapReport`. It should show the owning GLTF's
joint inventory; resolved values, provenance, confidence, and diagnostics; ambiguous candidates for
review; and controls for per-slot override, explicit absence, and the Auto toggle. It must preview
validation without mutating AVC topology and export the reviewed result as an MMS preset factory.

The panel must preserve `ComponentRef` surface forms, distinguish authored decisions from runtime
inference, and never serialize generated anchors or runtime-only reports.
