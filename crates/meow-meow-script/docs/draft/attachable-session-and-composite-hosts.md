# Attachable MMS sessions and composite hosts (draft)

Status: rough architectural sketch. This is not a committed protocol or public
API for 0.8.

## Motivation

A graphical Mittens process may already own a live MMS session containing:

- lexical variables and mutable tables;
- loaded functions and modules;
- callback registrations;
- live component handles;
- the RuntimeSpec and its operation bindings.

We want to attach a terminal to that existing session and evaluate or inspect
code in the same world. Attaching must not create a second evaluator with a
copied environment. The useful analogy is attaching a debugger to an existing
Node.js VM: the client connects to the running language session and observes or
controls that session's existing state.

The initial target is a graphical Mittens application. A later headless
Mittens process should expose the same shape, and standalone MMS embedders may
use it without Mittens.

## Rough model

```text
terminal / editor / debugger client
              |
       SessionClient protocol
              |
     one persistent MMS session
   scopes + heap + tables + callbacks
              |
       one logical AppHost
       /        |         \
 MittensHost  FilesHost  ShellHost
 ECS/render   files/watch stdout/process
```

There are two separate composition questions:

1. **Host services:** a session may need engine, filesystem, terminal, or other
   capabilities. These can be separate implementations behind one logical
   composite host.
2. **Attached clients:** multiple frontends may submit evaluation, inspection,
   and subscription requests to the one session authority.

An attached terminal is usually a `SessionClient`, not a second host and not a
second MMS session. Whether terminal-facing operations such as stdout or shell
execution are provided by the composite host is an independent capability
decision.

## One logical host, several services

MMS should continue to see one host boundary per session. An application can
compose that boundary from independently implemented services:

```rust,ignore
enum AppBinding {
    Mittens(MittensBinding),
    Filesystem(FilesystemBinding),
    Shell(ShellBinding),
}

struct AppHost {
    mittens: MittensHost,
    filesystem: FilesystemHost,
    shell: ShellHost,
}
```

The composite host advertises the combined capabilities and routes each
RuntimeSpec operation binding to exactly one implementation. The MMS session
still has one `HostContext`, one handle-ownership domain, and one deterministic
host boundary.

Filesystem and process access must remain explicit capabilities. Attaching a
terminal must not silently grant them.

## One session authority, several clients

The persistent session should have one serialized owner, provisionally called
the session authority or session worker. Clients send it requests rather than
borrowing the evaluator directly.

Possible high-level requests:

```text
Attach
Eval(source, source_path?)
Inspect(value_or_handle)
SubscribeOutput
UnsubscribeOutput
Interrupt(request_id)
Detach
```

Engine signals enqueue opaque callback invocations for this same authority.
Terminal evaluations and engine callbacks therefore share the session's real
tables, closures, modules, and component handles.

All evaluator entry must be serialized initially. A callback and a terminal
evaluation must never mutate the MMS heap concurrently. Fairness, interruption,
and pause/step behavior can be designed later.

## Important invariants

- There is exactly one authoritative MMS heap and lexical environment for the
  running application session.
- Attaching does not clone, reset, or replace that environment.
- Every RuntimeSpec operation resolves to exactly one host binding.
- Live handles stay opaque and are validated by the session/host ownership
  boundary.
- Clients exchange transport-safe values, diagnostics, and handle summaries;
  they do not receive Rust references or engine internals.
- Callback execution and client evaluation are serialized through the same
  session authority.
- Client attachment does not expand host capabilities.
- Client disconnect does not terminate the application or its MMS session.
- Application/session shutdown invalidates clients and callback handles in a
  defined way.

## Graphical and headless shapes

### Initially: graphical Mittens

The window loop owns the Universe and persistent MMS session. After an engine
update it services queued MMS callbacks. An attach endpoint lets a terminal
submit work to that same session between or alongside frame-driven requests.

Long-running terminal evaluation must not indefinitely block rendering. The
first implementation may impose a small work budget or only process one
session request at a safe frame boundary.

### Later: headless Mittens

A headless loop owns the same session authority and composite host without a
window or renderer. The terminal protocol should not depend on graphical
windowing APIs.

### Standalone MMS

A non-Mittens embedder may provide its own composite host and session loop.
The client/session protocol and callback ownership should remain usable without
engine component types.

## Transport is deliberately undecided

Possible transports include an in-process channel, stdin/stdout, a Unix domain
socket, or an authenticated local TCP/WebSocket endpoint. The logical protocol
should not require one transport.

The Node.js debugger comparison describes attachment to the same running
language state; it does not imply compatibility with the Node inspector wire
protocol.

## Possible incremental path

1. Finish the generic persistent `SessionClient`/session-worker boundary.
2. Move the current Mittens callback queue and frame servicing behind it.
3. Add an in-process attach client used by tests and the existing terminal
   REPL.
4. Add a local transport and a small external terminal client.
5. Allow the same session worker to run in headless Mittens.
6. Consider debugger features such as pause, stepping, breakpoints, structured
   inspection, and event subscriptions separately.

## Open questions

- Does Mittens own the attach endpoint, or does MMS provide a reusable server?
- What is the minimum stable `SessionClient` protocol for 0.8, if any?
- How are multiple attached clients ordered and identified?
- What evaluation budget prevents a client from stalling a graphical frame?
- How are stdout, diagnostics, and emitted component trees multiplexed?
- Can a client request pause/step semantics, or is the first version only a
  live REPL?
- How are filesystem and process capabilities configured and audited?
- Should an attached client be read-only by default?
- How are reconnects, stale handles, application reloads, and session restart
  represented?
