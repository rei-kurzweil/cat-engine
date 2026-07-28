# XR performance report

- Preset: `avatar_no_spring_mirror`
- Avatar / XR control: on
- Mirror: on
- Secondary motion: off
- Spring-bone visualization: off
- Warm-up requested: 5.000 s
- Sample requested: 60.000 s

## Headset frame results

- Sampled headset frames: 4096
- Elapsed: 60.004 s
- Arithmetic average FPS: 68.262
- Mean headset frame time: 14.649 ms
- Median headset frame time: 13.854 ms
- p95 headset frame time: 18.450 ms
- p99 headset frame time: 26.703 ms
- Minimum headset frame time: 12.369 ms
- Maximum headset frame time: 330.421 ms
- Runtime display interval: 12.153 ms
- Frames exceeding display interval: 4096 (100.00%)
- Runtime dropped frames: unavailable
- Runtime reprojected frames: unavailable

## Environment

- Build profile: release
- GPU / device: NVIDIA GeForce GTX 1080
- OpenXR runtime: SteamVR/OpenXR (2.12.14)
- Headset target refresh rate: 82.286 Hz
- Render extent: 1868 × 1868
- MSAA: 4x

## CPU timing

- Mean Update before XR: 6.024 ms
- Mean Final command processing: 0.001 ms
- Mean Secondary-motion simulation: 0.000 ms
- Mean Spring transform propagation: 0.000 ms
- Mean Spring visualization: 0.002 ms
- Mean Post-secondary skinning: 0.261 ms
- Mean Post-pose/layout command flush: 0.068 ms
- Mean Render preparation: 0.014 ms
- Mean Total XR frame: 8.592 ms
- Mean wait_frame: 0.117 ms
- Mean Eye render: 6.006 ms
- Mean Swapchain copy: 1.331 ms
- Mean Frame submit: 0.740 ms

## Detailed renderer / deformation counters

- Vulkan queue submissions: 20480 total, 5.000 per headset frame
- CPU fence waits: 4096 total, 1.000 per headset frame
- CPU queue-idle waits: 0 total, 0.000 per headset frame
- Mirror captures: 8192 total, 2.000 per headset frame
- XR eyes rendered: 8192 total, 2.000 per headset frame
- Deformation dispatches: 4096 total, 1.000 per headset frame
- Deformation jobs: 65536 total, 16.000 per headset frame
- Deformation workgroups: 2244608 total, 548.000 per headset frame
- Dirty deformation vertices: 141582336 total, 34566.000 per headset frame
- Bone upload bytes: 541065216 total, 132096.000 per headset frame
- Job upload bytes: 20054016 total, 4896.000 per headset frame
- Morph-weight upload bytes: 0 total, 0.000 per headset frame

Mirror GPU time, per-eye GPU time, deformation allocations, and per-view draw counts: unavailable.
