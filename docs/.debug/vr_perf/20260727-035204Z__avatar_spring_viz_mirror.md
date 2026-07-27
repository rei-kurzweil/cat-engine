# XR performance report

- Preset: `avatar_spring_viz_mirror`
- Avatar / XR control: on
- Mirror: on
- Secondary motion: on
- Spring-bone visualization: on
- Warm-up requested: 5.000 s
- Sample requested: 10.000 s

## Headset frame results

- Sampled headset frames: 94
- Elapsed: 10.085 s
- Arithmetic average FPS: 9.321
- Mean headset frame time: 107.286 ms
- Median headset frame time: 104.347 ms
- p95 headset frame time: 120.096 ms
- p99 headset frame time: 123.859 ms
- Minimum headset frame time: 99.355 ms
- Maximum headset frame time: 123.859 ms
- Runtime display interval: 13.239 ms
- Frames exceeding display interval: 94 (100.00%)
- Runtime dropped frames: unavailable
- Runtime reprojected frames: unavailable

## Environment

- Build profile: release
- GPU / device: NVIDIA GeForce GTX 1080
- OpenXR runtime: SteamVR/OpenXR (2.12.14)
- Headset target refresh rate: 75.536 Hz
- Render extent: 1868 × 1868
- MSAA: 4x

## CPU timing

- Mean Update before XR: 77.951 ms
- Mean Final command processing: 0.001 ms
- Mean Secondary-motion simulation: 0.078 ms
- Mean Spring transform propagation: 5.324 ms
- Mean Spring visualization: 0.083 ms
- Mean Post-secondary skinning: 0.294 ms
- Mean Post-pose/layout command flush: 67.192 ms
- Mean Render preparation: 0.010 ms
- Mean Total XR frame: 29.306 ms
- Mean wait_frame: 0.016 ms
- Mean Eye render: 20.721 ms
- Mean Swapchain copy: 0.205 ms
- Mean Frame submit: 8.049 ms

## Detailed renderer / deformation counters

- Vulkan queue submissions: 846 total, 9.000 per headset frame
- CPU fence waits: 752 total, 8.000 per headset frame
- CPU queue-idle waits: 94 total, 1.000 per headset frame
- Mirror captures: 564 total, 6.000 per headset frame
- XR eyes rendered: 188 total, 2.000 per headset frame
- Deformation dispatches: 94 total, 1.000 per headset frame
- Deformation jobs: 1504 total, 16.000 per headset frame
- Deformation workgroups: 51512 total, 548.000 per headset frame
- Dirty deformation vertices: 3249204 total, 34566.000 per headset frame
- Bone upload bytes: 12417024 total, 132096.000 per headset frame
- Job upload bytes: 460224 total, 4896.000 per headset frame
- Morph-weight upload bytes: 0 total, 0.000 per headset frame

Mirror GPU time, per-eye GPU time, deformation allocations, and per-view draw counts: unavailable.
