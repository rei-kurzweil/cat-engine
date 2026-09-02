# Laptop rendering server

Date: 2026-09-02

Status: blocked for full-power laptop/GPU validation by the need for a new
laptop battery. Protocol design, localhost testing, and low-power integration
work remain available.

[Back to the desktop workbench](README.md)

## Outcome

Run a second Mittens instance on the laptop as a presentation/rendering worker.
The VR computer remains authoritative for interaction and simulation, while the
laptop reconstructs the scene, follows a studio camera, renders the broadcast
view, and exposes that view to OBS. The VR computer should no longer draw the
scene a second time from the studio camera merely for broadcasting.

This is state/intent offload, not remote video streaming: the machines exchange
the scene snapshot and subsequent changes, and the laptop draws its own image.

## Dependency chain

```text
replacement laptop battery
  -> sustained full-power laptop GPU
  -> realistic two-machine performance/thermal validation

versioned UDP transport exposed to MMS
  -> initial scene snapshot + ordered intent/state stream
  -> remote Mittens scene reconstruction
  -> studio-camera render on laptop
  -> OBS capture/output
```

The hardware branch blocks the final proof, but it does not block defining or
testing the software branch on loopback or at reduced performance.

## Required contract

The transport must define:

- explicit sender/listener lifecycle and MMS-visible configuration;
- protocol version, session identity, monotonically increasing sequence, and
  simulation/frame timestamp;
- one authoritative initial snapshot, followed by ordered state changes or
  intents;
- how assets are identified and made available on both machines;
- loss, duplication, reordering, reconnect, and late-join behavior;
- periodic checksum or snapshot recovery so a lost UDP packet cannot leave the
  render worker permanently divergent;
- bounded queues and a policy for coalescing replaceable high-rate updates;
- bind/interface configuration and a trusted-LAN security boundary.

Raw ECS implementation details should not accidentally become the network
protocol. Use stable identifiers and a versioned wire schema.

## Milestones

- [ ] Write the smallest MMS authoring surface for a UDP sender/listener and
      document whether handlers receive packets, decoded events, or both.
- [ ] Inventory which current intent/event payloads can be serialized without
      process-local component handles leaking onto the wire.
- [ ] Choose snapshot ownership and asset synchronization rules.
- [ ] Build a localhost proof: one process publishes a tiny scene snapshot and
      transform updates; a second process reconstructs them.
- [ ] Add sequence-gap detection, periodic recovery, reconnect, and late join.
- [ ] Add a studio-camera worker mode that does not acquire VR hardware or
      become the simulation authority.
- [ ] Expose a stable window/texture/output path that OBS can capture.
- [ ] Record VR-host GPU frame time with and without its local studio-camera
      render to prove the work was actually removed.
- [ ] After battery replacement, run sustained two-machine validation at the
      laptop GPU's intended power level and record thermals, frame pacing,
      network loss, and end-to-end broadcast latency.

## First vertical slice

Use a deliberately small proof before attempting arbitrary-world replication:

1. sender publishes a versioned snapshot containing one transform and one
   renderable whose asset is already installed on both machines;
2. sender publishes transform changes with sequence numbers;
3. worker detects a deliberately dropped packet and requests/accepts a fresh
   snapshot;
4. worker renders a studio camera to a normal capturable window;
5. measurements show that the VR host no longer submits the studio-camera
   render view.

## Related engine work

- [MMS event payloads and runtime attach](../task/mms-event-payloads-and-runtime-attach.md)
- [Input → intent → data flow](../spec/input-intent-data-flow.md)
- [Event-signal pipelines](../draft/event-signal-pipelines.md)
- [Render-to-texture](../spec/render-to-texture.md)
- [Render stream single source](../task/render-stream-single-source.md)
- [Renderer optimisation epic](../task/epic/renderer_optimisation.md)

Existing eye-tracking UDP code is evidence that the engine can own nonblocking
UDP sockets, but its fixed-purpose packet format is not a reusable scene/intent
replication protocol.
