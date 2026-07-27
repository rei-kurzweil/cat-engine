# XR performance report

- Preset: `avatar_spring_no_viz_mirror`
- Avatar / XR control: on
- Mirror: on
- Secondary motion: on
- Spring-bone visualization: off
- Warm-up requested: 5.000 s
- Sample requested: 10.000 s

## Headset frame results

- Sampled headset frames: 301
- Elapsed: 10.026 s
- Arithmetic average FPS: 30.021
- Mean headset frame time: 33.310 ms
- Median headset frame time: 27.533 ms
- p95 headset frame time: 40.433 ms
- p99 headset frame time: 40.860 ms
- Minimum headset frame time: 26.011 ms
- Maximum headset frame time: 54.114 ms
- Runtime display interval: 33.149 ms
- Frames exceeding display interval: 148 (49.17%)
- Runtime dropped frames: unavailable
- Runtime reprojected frames: unavailable

## Environment

- Build profile: release
- GPU / device: NVIDIA GeForce GTX 1080
- OpenXR runtime: SteamVR/OpenXR (2.12.14)
- Headset target refresh rate: 30.167 Hz
- Render extent: 1868 × 1868
- MSAA: 4x

## XR CPU timing

- Mean Total XR frame: 24.805 ms
- Mean wait_frame: 0.016 ms
- Mean Eye render: 17.289 ms
- Mean Swapchain copy: 0.154 ms
- Mean Frame submit: 7.076 ms

## Detailed renderer / deformation counters

- Vulkan queue submissions: 2709 total, 9.000 per headset frame
- CPU fence waits: 2408 total, 8.000 per headset frame
- CPU queue-idle waits: 301 total, 1.000 per headset frame
- Mirror captures: 1806 total, 6.000 per headset frame
- XR eyes rendered: 602 total, 2.000 per headset frame
- Deformation dispatches: 301 total, 1.000 per headset frame
- Deformation jobs: 4816 total, 16.000 per headset frame
- Deformation workgroups: 164948 total, 548.000 per headset frame
- Dirty deformation vertices: 10404366 total, 34566.000 per headset frame
- Bone upload bytes: 39760896 total, 132096.000 per headset frame
- Job upload bytes: 1473696 total, 4896.000 per headset frame
- Morph-weight upload bytes: 0 total, 0.000 per headset frame

Mirror GPU time, per-eye GPU time, deformation allocations, and per-view draw counts: unavailable.
