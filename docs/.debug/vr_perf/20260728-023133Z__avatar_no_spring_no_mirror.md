# XR performance report

- Preset: `avatar_no_spring_no_mirror`
- Avatar / XR control: on
- Mirror: off
- Secondary motion: off
- Spring-bone visualization: off
- Warm-up requested: 5.000 s
- Sample requested: 60.000 s

## Headset frame results

- Sampled headset frames: 4499
- Elapsed: 60.000 s
- Arithmetic average FPS: 74.983
- Mean headset frame time: 13.336 ms
- Median headset frame time: 13.332 ms
- p95 headset frame time: 14.060 ms
- p99 headset frame time: 14.515 ms
- Minimum headset frame time: 10.058 ms
- Maximum headset frame time: 16.980 ms
- Runtime display interval: 11.114 ms
- Frames exceeding display interval: 4496 (99.93%)
- Runtime dropped frames: unavailable
- Runtime reprojected frames: unavailable

## Environment

- Build profile: release
- GPU / device: NVIDIA GeForce GTX 1080
- OpenXR runtime: SteamVR/OpenXR (2.12.14)
- Headset target refresh rate: 89.980 Hz
- Render extent: 1868 × 1868
- MSAA: 4x

## CPU timing

- Mean Update before XR: 5.746 ms
- Mean Final command processing: 0.001 ms
- Mean Secondary-motion simulation: 0.000 ms
- Mean Spring transform propagation: 0.000 ms
- Mean Spring visualization: 0.001 ms
- Mean Post-secondary skinning: 0.261 ms
- Mean Post-pose/layout command flush: 0.001 ms
- Mean Render preparation: 0.008 ms
- Mean Total XR frame: 7.563 ms
- Mean wait_frame: 1.864 ms
- Mean Eye render: 2.266 ms
- Mean Swapchain copy: 1.228 ms
- Mean Frame submit: 1.820 ms

## Detailed renderer / deformation counters

- Vulkan queue submissions: 13497 total, 3.000 per headset frame
- CPU fence waits: 4499 total, 1.000 per headset frame
- CPU queue-idle waits: 0 total, 0.000 per headset frame
- Mirror captures: 0 total, 0.000 per headset frame
- XR eyes rendered: 8998 total, 2.000 per headset frame
- Deformation dispatches: 4499 total, 1.000 per headset frame
- Deformation jobs: 71984 total, 16.000 per headset frame
- Deformation workgroups: 2465452 total, 548.000 per headset frame
- Dirty deformation vertices: 155512434 total, 34566.000 per headset frame
- Bone upload bytes: 594299904 total, 132096.000 per headset frame
- Job upload bytes: 22027104 total, 4896.000 per headset frame
- Morph-weight upload bytes: 0 total, 0.000 per headset frame

Mirror GPU time, per-eye GPU time, deformation allocations, and per-view draw counts: unavailable.
