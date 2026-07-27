# XR performance report

- Preset: `avatar_spring_no_viz_mirror`
- Avatar / XR control: on
- Mirror: on
- Secondary motion: on
- Spring-bone visualization: off
- Warm-up requested: 5.000 s
- Sample requested: 10.000 s

## Headset frame results

- Sampled headset frames: 410
- Elapsed: 10.006 s
- Arithmetic average FPS: 40.976
- Mean headset frame time: 24.404 ms
- Median headset frame time: 26.294 ms
- p95 headset frame time: 27.538 ms
- p99 headset frame time: 31.034 ms
- Minimum headset frame time: 19.106 ms
- Maximum headset frame time: 43.232 ms
- Runtime display interval: 20.596 ms
- Frames exceeding display interval: 286 (69.76%)
- Runtime dropped frames: unavailable
- Runtime reprojected frames: unavailable

## Environment

- Build profile: release
- GPU / device: NVIDIA GeForce GTX 1080
- OpenXR runtime: SteamVR/OpenXR (2.12.14)
- Headset target refresh rate: 48.553 Hz
- Render extent: 1868 × 1868
- MSAA: 4x

## CPU timing

- Mean Update before XR: 10.297 ms
- Mean Final command processing: 0.001 ms
- Mean Secondary-motion simulation: 0.041 ms
- Mean Spring transform propagation: 5.003 ms
- Mean Spring visualization: 0.022 ms
- Mean Post-secondary skinning: 0.271 ms
- Mean Post-pose/layout command flush: 0.001 ms
- Mean Render preparation: 0.009 ms
- Mean Total XR frame: 14.079 ms
- Mean wait_frame: 0.016 ms
- Mean Eye render: 9.486 ms
- Mean Swapchain copy: 0.160 ms
- Mean Frame submit: 4.112 ms

## Detailed renderer / deformation counters

- Vulkan queue submissions: 2050 total, 5.000 per headset frame
- CPU fence waits: 1640 total, 4.000 per headset frame
- CPU queue-idle waits: 410 total, 1.000 per headset frame
- Mirror captures: 820 total, 2.000 per headset frame
- XR eyes rendered: 820 total, 2.000 per headset frame
- Deformation dispatches: 410 total, 1.000 per headset frame
- Deformation jobs: 6560 total, 16.000 per headset frame
- Deformation workgroups: 224680 total, 548.000 per headset frame
- Dirty deformation vertices: 14172060 total, 34566.000 per headset frame
- Bone upload bytes: 54159360 total, 132096.000 per headset frame
- Job upload bytes: 2007360 total, 4896.000 per headset frame
- Morph-weight upload bytes: 0 total, 0.000 per headset frame

Mirror GPU time, per-eye GPU time, deformation allocations, and per-view draw counts: unavailable.
