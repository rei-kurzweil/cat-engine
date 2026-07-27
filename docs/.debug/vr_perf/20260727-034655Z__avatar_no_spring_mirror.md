# XR performance report

- Preset: `avatar_no_spring_mirror`
- Avatar / XR control: on
- Mirror: on
- Secondary motion: off
- Spring-bone visualization: off
- Warm-up requested: 5.000 s
- Sample requested: 10.000 s

## Headset frame results

- Sampled headset frames: 375
- Elapsed: 10.026 s
- Arithmetic average FPS: 37.402
- Mean headset frame time: 26.736 ms
- Median headset frame time: 26.610 ms
- p95 headset frame time: 27.474 ms
- p99 headset frame time: 28.375 ms
- Minimum headset frame time: 25.165 ms
- Maximum headset frame time: 40.435 ms
- Runtime display interval: 22.341 ms
- Frames exceeding display interval: 375 (100.00%)
- Runtime dropped frames: unavailable
- Runtime reprojected frames: unavailable

## Environment

- Build profile: release
- GPU / device: NVIDIA GeForce GTX 1080
- OpenXR runtime: SteamVR/OpenXR (2.12.14)
- Headset target refresh rate: 44.761 Hz
- Render extent: 1868 × 1868
- MSAA: 4x

## CPU timing

- Mean Update before XR: 5.019 ms
- Mean Final command processing: 0.001 ms
- Mean Secondary-motion simulation: 0.000 ms
- Mean Spring transform propagation: 0.000 ms
- Mean Spring visualization: 0.001 ms
- Mean Post-secondary skinning: 0.242 ms
- Mean Post-pose/layout command flush: 0.001 ms
- Mean Render preparation: 0.009 ms
- Mean Total XR frame: 21.689 ms
- Mean wait_frame: 0.015 ms
- Mean Eye render: 17.425 ms
- Mean Swapchain copy: 0.145 ms
- Mean Frame submit: 3.823 ms

## Detailed renderer / deformation counters

- Vulkan queue submissions: 3375 total, 9.000 per headset frame
- CPU fence waits: 3000 total, 8.000 per headset frame
- CPU queue-idle waits: 375 total, 1.000 per headset frame
- Mirror captures: 2250 total, 6.000 per headset frame
- XR eyes rendered: 750 total, 2.000 per headset frame
- Deformation dispatches: 375 total, 1.000 per headset frame
- Deformation jobs: 6000 total, 16.000 per headset frame
- Deformation workgroups: 205500 total, 548.000 per headset frame
- Dirty deformation vertices: 12962250 total, 34566.000 per headset frame
- Bone upload bytes: 49536000 total, 132096.000 per headset frame
- Job upload bytes: 1836000 total, 4896.000 per headset frame
- Morph-weight upload bytes: 0 total, 0.000 per headset frame

Mirror GPU time, per-eye GPU time, deformation allocations, and per-view draw counts: unavailable.
