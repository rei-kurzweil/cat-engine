# XR performance report

- Preset: `avatar_no_spring_mirror`
- Avatar / XR control: on
- Mirror: on
- Secondary motion: off
- Spring-bone visualization: off
- Warm-up requested: 5.000 s
- Sample requested: 10.000 s

## Headset frame results

- Sampled headset frames: 373
- Elapsed: 10.000 s
- Arithmetic average FPS: 37.300
- Mean headset frame time: 26.810 ms
- Median headset frame time: 26.713 ms
- p95 headset frame time: 27.436 ms
- p99 headset frame time: 39.149 ms
- Minimum headset frame time: 24.352 ms
- Maximum headset frame time: 40.358 ms
- Runtime display interval: 22.461 ms
- Frames exceeding display interval: 373 (100.00%)
- Runtime dropped frames: unavailable
- Runtime reprojected frames: unavailable

## Environment

- Build profile: release
- GPU / device: NVIDIA GeForce GTX 1080
- OpenXR runtime: SteamVR/OpenXR (2.12.14)
- Headset target refresh rate: 44.523 Hz
- Render extent: 1868 × 1868
- MSAA: 4x

## XR CPU timing

- Mean Total XR frame: 22.815 ms
- Mean wait_frame: 0.014 ms
- Mean Eye render: 17.101 ms
- Mean Swapchain copy: 0.145 ms
- Mean Frame submit: 5.279 ms

## Detailed renderer / deformation counters

- Vulkan queue submissions: 3357 total, 9.000 per headset frame
- CPU fence waits: 2984 total, 8.000 per headset frame
- CPU queue-idle waits: 373 total, 1.000 per headset frame
- Mirror captures: 2238 total, 6.000 per headset frame
- XR eyes rendered: 746 total, 2.000 per headset frame
- Deformation dispatches: 372 total, 0.997 per headset frame
- Deformation jobs: 5952 total, 15.957 per headset frame
- Deformation workgroups: 203856 total, 546.531 per headset frame
- Dirty deformation vertices: 12858552 total, 34473.330 per headset frame
- Bone upload bytes: 49139712 total, 131741.855 per headset frame
- Job upload bytes: 1821312 total, 4882.874 per headset frame
- Morph-weight upload bytes: 0 total, 0.000 per headset frame

Mirror GPU time, per-eye GPU time, deformation allocations, and per-view draw counts: unavailable.
