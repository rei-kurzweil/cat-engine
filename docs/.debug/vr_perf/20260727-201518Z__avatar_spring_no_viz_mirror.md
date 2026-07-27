# XR performance report

- Preset: `avatar_spring_no_viz_mirror`
- Avatar / XR control: on
- Mirror: on
- Secondary motion: on
- Spring-bone visualization: off
- Warm-up requested: 5.000 s
- Sample requested: 10.000 s

## Headset frame results

- Sampled headset frames: 376
- Elapsed: 10.000 s
- Arithmetic average FPS: 37.599
- Mean headset frame time: 26.597 ms
- Median headset frame time: 26.597 ms
- p95 headset frame time: 27.529 ms
- p99 headset frame time: 27.691 ms
- Minimum headset frame time: 21.515 ms
- Maximum headset frame time: 39.722 ms
- Runtime display interval: 22.193 ms
- Frames exceeding display interval: 371 (98.67%)
- Runtime dropped frames: unavailable
- Runtime reprojected frames: unavailable

## Environment

- Build profile: release
- GPU / device: NVIDIA GeForce GTX 1080
- OpenXR runtime: SteamVR/OpenXR (2.12.14)
- Headset target refresh rate: 45.060 Hz
- Render extent: 1868 × 1868
- MSAA: 4x

## CPU timing

- Mean Update before XR: 11.318 ms
- Mean Final command processing: 0.001 ms
- Mean Secondary-motion simulation: 0.046 ms
- Mean Spring transform propagation: 5.512 ms
- Mean Spring visualization: 0.024 ms
- Mean Post-secondary skinning: 0.298 ms
- Mean Post-pose/layout command flush: 0.002 ms
- Mean Render preparation: 0.010 ms
- Mean Total XR frame: 15.248 ms
- Mean wait_frame: 0.017 ms
- Mean Eye render: 10.048 ms
- Mean Swapchain copy: 0.149 ms
- Mean Frame submit: 4.726 ms

## Detailed renderer / deformation counters

- Vulkan queue submissions: 1880 total, 5.000 per headset frame
- CPU fence waits: 1504 total, 4.000 per headset frame
- CPU queue-idle waits: 376 total, 1.000 per headset frame
- Mirror captures: 752 total, 2.000 per headset frame
- XR eyes rendered: 752 total, 2.000 per headset frame
- Deformation dispatches: 376 total, 1.000 per headset frame
- Deformation jobs: 6016 total, 16.000 per headset frame
- Deformation workgroups: 206048 total, 548.000 per headset frame
- Dirty deformation vertices: 12996816 total, 34566.000 per headset frame
- Bone upload bytes: 49668096 total, 132096.000 per headset frame
- Job upload bytes: 1840896 total, 4896.000 per headset frame
- Morph-weight upload bytes: 0 total, 0.000 per headset frame

Mirror GPU time, per-eye GPU time, deformation allocations, and per-view draw counts: unavailable.
