# XR performance report

- Preset: `avatar_spring_no_viz_mirror`
- Avatar / XR control: on
- Mirror: on
- Secondary motion: on
- Spring-bone visualization: off
- Warm-up requested: 5.000 s
- Sample requested: 10.000 s

## Headset frame results

- Sampled headset frames: 250
- Elapsed: 10.013 s
- Arithmetic average FPS: 24.967
- Mean headset frame time: 40.053 ms
- Median headset frame time: 40.000 ms
- p95 headset frame time: 40.689 ms
- p99 headset frame time: 41.563 ms
- Minimum headset frame time: 38.031 ms
- Maximum headset frame time: 53.653 ms
- Runtime display interval: 44.489 ms
- Frames exceeding display interval: 1 (0.40%)
- Runtime dropped frames: unavailable
- Runtime reprojected frames: unavailable

## Environment

- Build profile: release
- GPU / device: NVIDIA GeForce GTX 1080
- OpenXR runtime: SteamVR/OpenXR (2.12.14)
- Headset target refresh rate: 22.478 Hz
- Render extent: 1868 × 1868
- MSAA: 4x

## CPU timing

- Mean Update before XR: 10.061 ms
- Mean Final command processing: 0.001 ms
- Mean Secondary-motion simulation: 0.054 ms
- Mean Spring transform propagation: 5.041 ms
- Mean Spring visualization: 0.021 ms
- Mean Post-secondary skinning: 0.263 ms
- Mean Post-pose/layout command flush: 0.001 ms
- Mean Render preparation: 0.009 ms
- Mean Total XR frame: 29.961 ms
- Mean wait_frame: 0.015 ms
- Mean Eye render: 18.368 ms
- Mean Swapchain copy: 0.175 ms
- Mean Frame submit: 11.106 ms

## Detailed renderer / deformation counters

- Vulkan queue submissions: 2250 total, 9.000 per headset frame
- CPU fence waits: 2000 total, 8.000 per headset frame
- CPU queue-idle waits: 250 total, 1.000 per headset frame
- Mirror captures: 1500 total, 6.000 per headset frame
- XR eyes rendered: 500 total, 2.000 per headset frame
- Deformation dispatches: 250 total, 1.000 per headset frame
- Deformation jobs: 4000 total, 16.000 per headset frame
- Deformation workgroups: 137000 total, 548.000 per headset frame
- Dirty deformation vertices: 8641500 total, 34566.000 per headset frame
- Bone upload bytes: 33024000 total, 132096.000 per headset frame
- Job upload bytes: 1224000 total, 4896.000 per headset frame
- Morph-weight upload bytes: 0 total, 0.000 per headset frame

Mirror GPU time, per-eye GPU time, deformation allocations, and per-view draw counts: unavailable.
