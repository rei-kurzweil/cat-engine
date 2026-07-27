# XR performance report

- Preset: `avatar_spring_viz_mirror`
- Avatar / XR control: on
- Mirror: on
- Secondary motion: on
- Spring-bone visualization: on
- Warm-up requested: 5.000 s
- Sample requested: 10.000 s

## Headset frame results

- Sampled headset frames: 84
- Elapsed: 10.013 s
- Arithmetic average FPS: 8.389
- Mean headset frame time: 119.204 ms
- Median headset frame time: 119.991 ms
- p95 headset frame time: 120.212 ms
- p99 headset frame time: 120.372 ms
- Minimum headset frame time: 90.797 ms
- Maximum headset frame time: 120.372 ms
- Runtime display interval: 11.243 ms
- Frames exceeding display interval: 84 (100.00%)
- Runtime dropped frames: unavailable
- Runtime reprojected frames: unavailable

## Environment

- Build profile: release
- GPU / device: NVIDIA GeForce GTX 1080
- OpenXR runtime: SteamVR/OpenXR (2.12.14)
- Headset target refresh rate: 88.941 Hz
- Render extent: 1868 × 1868
- MSAA: 4x

## CPU timing

- Mean Update before XR: 77.110 ms
- Mean Final command processing: 0.001 ms
- Mean Secondary-motion simulation: 0.082 ms
- Mean Spring transform propagation: 5.311 ms
- Mean Spring visualization: 0.082 ms
- Mean Post-secondary skinning: 0.293 ms
- Mean Post-pose/layout command flush: 66.345 ms
- Mean Render preparation: 0.010 ms
- Mean Total XR frame: 42.062 ms
- Mean wait_frame: 0.017 ms
- Mean Eye render: 11.958 ms
- Mean Swapchain copy: 0.172 ms
- Mean Frame submit: 29.589 ms

## Detailed renderer / deformation counters

- Vulkan queue submissions: 420 total, 5.000 per headset frame
- CPU fence waits: 336 total, 4.000 per headset frame
- CPU queue-idle waits: 84 total, 1.000 per headset frame
- Mirror captures: 168 total, 2.000 per headset frame
- XR eyes rendered: 168 total, 2.000 per headset frame
- Deformation dispatches: 84 total, 1.000 per headset frame
- Deformation jobs: 1344 total, 16.000 per headset frame
- Deformation workgroups: 46032 total, 548.000 per headset frame
- Dirty deformation vertices: 2903544 total, 34566.000 per headset frame
- Bone upload bytes: 11096064 total, 132096.000 per headset frame
- Job upload bytes: 411264 total, 4896.000 per headset frame
- Morph-weight upload bytes: 0 total, 0.000 per headset frame

Mirror GPU time, per-eye GPU time, deformation allocations, and per-view draw counts: unavailable.
