# XR performance report

- Preset: `avatar_no_spring_no_mirror`
- Avatar / XR control: on
- Mirror: off
- Secondary motion: off
- Spring-bone visualization: off
- Warm-up requested: 5.000 s
- Sample requested: 10.000 s

## Headset frame results

- Sampled headset frames: 748
- Elapsed: 10.000 s
- Arithmetic average FPS: 74.799
- Mean headset frame time: 13.369 ms
- Median headset frame time: 13.338 ms
- p95 headset frame time: 13.616 ms
- p99 headset frame time: 14.705 ms
- Minimum headset frame time: 11.585 ms
- Maximum headset frame time: 26.129 ms
- Runtime display interval: 11.141 ms
- Frames exceeding display interval: 748 (100.00%)
- Runtime dropped frames: unavailable
- Runtime reprojected frames: unavailable

## Environment

- Build profile: release
- GPU / device: NVIDIA GeForce GTX 1080
- OpenXR runtime: SteamVR/OpenXR (2.12.14)
- Headset target refresh rate: 89.760 Hz
- Render extent: 1868 × 1868
- MSAA: 4x

## XR CPU timing

- Mean Total XR frame: 8.355 ms
- Mean wait_frame: 2.513 ms
- Mean Eye render: 3.899 ms
- Mean Swapchain copy: 0.152 ms
- Mean Frame submit: 1.485 ms

## Detailed renderer / deformation counters

- Vulkan queue submissions: 2244 total, 3.000 per headset frame
- CPU fence waits: 1496 total, 2.000 per headset frame
- CPU queue-idle waits: 748 total, 1.000 per headset frame
- Mirror captures: 0 total, 0.000 per headset frame
- XR eyes rendered: 1496 total, 2.000 per headset frame
- Deformation dispatches: 748 total, 1.000 per headset frame
- Deformation jobs: 11968 total, 16.000 per headset frame
- Deformation workgroups: 409904 total, 548.000 per headset frame
- Dirty deformation vertices: 25855368 total, 34566.000 per headset frame
- Bone upload bytes: 98807808 total, 132096.000 per headset frame
- Job upload bytes: 3662208 total, 4896.000 per headset frame
- Morph-weight upload bytes: 0 total, 0.000 per headset frame

Mirror GPU time, per-eye GPU time, deformation allocations, and per-view draw counts: unavailable.
