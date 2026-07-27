# XR performance report

- Preset: `avatar_spring_viz_mirror`
- Avatar / XR control: on
- Mirror: on
- Secondary motion: on
- Spring-bone visualization: on
- Warm-up requested: 5.000 s
- Sample requested: 10.000 s

## Headset frame results

- Sampled headset frames: 98
- Elapsed: 10.088 s
- Arithmetic average FPS: 9.715
- Mean headset frame time: 102.934 ms
- Median headset frame time: 102.351 ms
- p95 headset frame time: 106.783 ms
- p99 headset frame time: 141.331 ms
- Minimum headset frame time: 100.086 ms
- Maximum headset frame time: 141.331 ms
- Runtime display interval: 12.018 ms
- Frames exceeding display interval: 98 (100.00%)
- Runtime dropped frames: unavailable
- Runtime reprojected frames: unavailable

## Environment

- Build profile: release
- GPU / device: NVIDIA GeForce GTX 1080
- OpenXR runtime: SteamVR/OpenXR (2.12.14)
- Headset target refresh rate: 83.208 Hz
- Render extent: 1868 × 1868
- MSAA: 4x

## XR CPU timing

- Mean Total XR frame: 25.932 ms
- Mean wait_frame: 0.016 ms
- Mean Eye render: 20.067 ms
- Mean Swapchain copy: 0.177 ms
- Mean Frame submit: 5.332 ms

## Detailed renderer / deformation counters

- Vulkan queue submissions: 882 total, 9.000 per headset frame
- CPU fence waits: 784 total, 8.000 per headset frame
- CPU queue-idle waits: 98 total, 1.000 per headset frame
- Mirror captures: 588 total, 6.000 per headset frame
- XR eyes rendered: 196 total, 2.000 per headset frame
- Deformation dispatches: 98 total, 1.000 per headset frame
- Deformation jobs: 1568 total, 16.000 per headset frame
- Deformation workgroups: 53704 total, 548.000 per headset frame
- Dirty deformation vertices: 3387468 total, 34566.000 per headset frame
- Bone upload bytes: 12945408 total, 132096.000 per headset frame
- Job upload bytes: 479808 total, 4896.000 per headset frame
- Morph-weight upload bytes: 0 total, 0.000 per headset frame

Mirror GPU time, per-eye GPU time, deformation allocations, and per-view draw counts: unavailable.
