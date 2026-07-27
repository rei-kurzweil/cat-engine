# XR performance report

- Preset: `avatar_spring_viz_mirror`
- Avatar / XR control: on
- Mirror: on
- Secondary motion: on
- Spring-bone visualization: on
- Warm-up requested: 5.000 s
- Sample requested: 10.000 s

## Headset frame results

- Sampled headset frames: 97
- Elapsed: 10.051 s
- Arithmetic average FPS: 9.651
- Mean headset frame time: 103.619 ms
- Median headset frame time: 103.021 ms
- p95 headset frame time: 109.445 ms
- p99 headset frame time: 138.837 ms
- Minimum headset frame time: 97.958 ms
- Maximum headset frame time: 138.837 ms
- Runtime display interval: 12.027 ms
- Frames exceeding display interval: 97 (100.00%)
- Runtime dropped frames: unavailable
- Runtime reprojected frames: unavailable

## Environment

- Build profile: release
- GPU / device: NVIDIA GeForce GTX 1080
- OpenXR runtime: SteamVR/OpenXR (2.12.14)
- Headset target refresh rate: 83.143 Hz
- Render extent: 1868 × 1868
- MSAA: 4x

## XR CPU timing

- Mean Total XR frame: 25.937 ms
- Mean wait_frame: 0.016 ms
- Mean Eye render: 20.249 ms
- Mean Swapchain copy: 0.154 ms
- Mean Frame submit: 5.173 ms

## Detailed renderer / deformation counters

- Vulkan queue submissions: 873 total, 9.000 per headset frame
- CPU fence waits: 776 total, 8.000 per headset frame
- CPU queue-idle waits: 97 total, 1.000 per headset frame
- Mirror captures: 582 total, 6.000 per headset frame
- XR eyes rendered: 194 total, 2.000 per headset frame
- Deformation dispatches: 97 total, 1.000 per headset frame
- Deformation jobs: 1552 total, 16.000 per headset frame
- Deformation workgroups: 53156 total, 548.000 per headset frame
- Dirty deformation vertices: 3352902 total, 34566.000 per headset frame
- Bone upload bytes: 12813312 total, 132096.000 per headset frame
- Job upload bytes: 474912 total, 4896.000 per headset frame
- Morph-weight upload bytes: 0 total, 0.000 per headset frame

Mirror GPU time, per-eye GPU time, deformation allocations, and per-view draw counts: unavailable.
