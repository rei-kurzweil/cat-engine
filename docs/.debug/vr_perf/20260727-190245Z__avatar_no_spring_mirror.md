# XR performance report

- Preset: `avatar_no_spring_mirror`
- Avatar / XR control: on
- Mirror: on
- Secondary motion: off
- Spring-bone visualization: off
- Warm-up requested: 5.000 s
- Sample requested: 10.000 s

## Headset frame results

- Sampled headset frames: 455
- Elapsed: 10.013 s
- Arithmetic average FPS: 45.439
- Mean headset frame time: 22.007 ms
- Median headset frame time: 26.548 ms
- p95 headset frame time: 26.820 ms
- p99 headset frame time: 27.479 ms
- Minimum headset frame time: 14.216 ms
- Maximum headset frame time: 40.496 ms
- Runtime display interval: 18.388 ms
- Frames exceeding display interval: 267 (58.68%)
- Runtime dropped frames: unavailable
- Runtime reprojected frames: unavailable

## Environment

- Build profile: release
- GPU / device: NVIDIA GeForce GTX 1080
- OpenXR runtime: SteamVR/OpenXR (2.12.14)
- Headset target refresh rate: 54.382 Hz
- Render extent: 1868 × 1868
- MSAA: 4x

## CPU timing

- Mean Update before XR: 5.214 ms
- Mean Final command processing: 0.001 ms
- Mean Secondary-motion simulation: 0.000 ms
- Mean Spring transform propagation: 0.000 ms
- Mean Spring visualization: 0.002 ms
- Mean Post-secondary skinning: 0.247 ms
- Mean Post-pose/layout command flush: 0.001 ms
- Mean Render preparation: 0.009 ms
- Mean Total XR frame: 16.764 ms
- Mean wait_frame: 0.168 ms
- Mean Eye render: 9.567 ms
- Mean Swapchain copy: 0.149 ms
- Mean Frame submit: 6.583 ms

## Detailed renderer / deformation counters

- Vulkan queue submissions: 2275 total, 5.000 per headset frame
- CPU fence waits: 1820 total, 4.000 per headset frame
- CPU queue-idle waits: 455 total, 1.000 per headset frame
- Mirror captures: 910 total, 2.000 per headset frame
- XR eyes rendered: 910 total, 2.000 per headset frame
- Deformation dispatches: 455 total, 1.000 per headset frame
- Deformation jobs: 7280 total, 16.000 per headset frame
- Deformation workgroups: 249340 total, 548.000 per headset frame
- Dirty deformation vertices: 15727530 total, 34566.000 per headset frame
- Bone upload bytes: 60103680 total, 132096.000 per headset frame
- Job upload bytes: 2227680 total, 4896.000 per headset frame
- Morph-weight upload bytes: 0 total, 0.000 per headset frame

Mirror GPU time, per-eye GPU time, deformation allocations, and per-view draw counts: unavailable.
