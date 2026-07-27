# XR performance report

- Preset: `avatar_spring_no_viz_mirror`
- Avatar / XR control: on
- Mirror: on
- Secondary motion: on
- Spring-bone visualization: off
- Warm-up requested: 5.000 s
- Sample requested: 10.000 s

## Headset frame results

- Sampled headset frames: 251
- Elapsed: 10.040 s
- Arithmetic average FPS: 24.999
- Mean headset frame time: 40.001 ms
- Median headset frame time: 40.031 ms
- p95 headset frame time: 40.739 ms
- p99 headset frame time: 41.240 ms
- Minimum headset frame time: 37.908 ms
- Maximum headset frame time: 41.994 ms
- Runtime display interval: 44.444 ms
- Frames exceeding display interval: 0 (0.00%)
- Runtime dropped frames: unavailable
- Runtime reprojected frames: unavailable

## Environment

- Build profile: release
- GPU / device: NVIDIA GeForce GTX 1080
- OpenXR runtime: SteamVR/OpenXR (2.12.14)
- Headset target refresh rate: 22.500 Hz
- Render extent: 1868 × 1868
- MSAA: 4x

## XR CPU timing

- Mean Total XR frame: 29.119 ms
- Mean wait_frame: 0.016 ms
- Mean Eye render: 18.121 ms
- Mean Swapchain copy: 0.163 ms
- Mean Frame submit: 10.514 ms

## Detailed renderer / deformation counters

- Vulkan queue submissions: 2259 total, 9.000 per headset frame
- CPU fence waits: 2008 total, 8.000 per headset frame
- CPU queue-idle waits: 251 total, 1.000 per headset frame
- Mirror captures: 1506 total, 6.000 per headset frame
- XR eyes rendered: 502 total, 2.000 per headset frame
- Deformation dispatches: 251 total, 1.000 per headset frame
- Deformation jobs: 4016 total, 16.000 per headset frame
- Deformation workgroups: 137548 total, 548.000 per headset frame
- Dirty deformation vertices: 8676066 total, 34566.000 per headset frame
- Bone upload bytes: 33156096 total, 132096.000 per headset frame
- Job upload bytes: 1228896 total, 4896.000 per headset frame
- Morph-weight upload bytes: 0 total, 0.000 per headset frame

Mirror GPU time, per-eye GPU time, deformation allocations, and per-view draw counts: unavailable.
