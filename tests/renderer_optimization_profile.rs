//! Revision-comparison probe for `docs/task/render-stream-single-source.md`.
//!
//! Run this test by itself in release mode so the process-wide counting allocator
//! does not observe allocations from other tests:
//!
//! ```text
//! cargo test --release --test renderer_optimization_profile -- --ignored --nocapture --test-threads=1
//! ```

use mittens_engine::engine::ecs::ComponentId;
use mittens_engine::engine::graphics::primitives::{
    GpuRenderable, MaterialHandle, MeshHandle, Transform,
};
use mittens_engine::engine::graphics::visual_world::VisualWorld;
use slotmap::KeyData;
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Instant;

struct CountingAllocator;

static COUNTING: AtomicBool = AtomicBool::new(false);
static ALLOCATION_CALLS: AtomicU64 = AtomicU64::new(0);
static ALLOCATED_BYTES: AtomicU64 = AtomicU64::new(0);

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if COUNTING.load(Ordering::Relaxed) {
            ALLOCATION_CALLS.fetch_add(1, Ordering::Relaxed);
            ALLOCATED_BYTES.fetch_add(layout.size() as u64, Ordering::Relaxed);
        }
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        if COUNTING.load(Ordering::Relaxed) {
            ALLOCATION_CALLS.fetch_add(1, Ordering::Relaxed);
            ALLOCATED_BYTES.fetch_add(layout.size() as u64, Ordering::Relaxed);
        }
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        if COUNTING.load(Ordering::Relaxed) {
            ALLOCATION_CALLS.fetch_add(1, Ordering::Relaxed);
            ALLOCATED_BYTES.fetch_add(new_size as u64, Ordering::Relaxed);
        }
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

fn component_id(n: u64) -> ComponentId {
    KeyData::from_ffi(n).into()
}

fn renderable() -> GpuRenderable {
    GpuRenderable::new(MeshHandle::SQUARE, MaterialHandle::TOON_MESH)
}

fn build_workload(instance_count: usize, clipped: bool) -> VisualWorld {
    let mut visuals = VisualWorld::default();

    if clipped {
        let clip = visuals.register(
            component_id(1),
            renderable(),
            Transform::default(),
            [1.0, 1.0, 1.0, 1.0],
            1.0,
            false,
            false,
            false,
            false,
            false,
            0.0,
            None,
            3.0,
        );
        assert!(visuals.register_stencil_clip(clip, 0));
    }

    for index in 0..instance_count {
        let phase = index % 4;
        let alpha = if phase == 2 { 0.5 } else { 1.0 };
        let handle = visuals.register(
            component_id(index as u64 + 2),
            renderable(),
            Transform::default(),
            [1.0, 1.0, 1.0, alpha],
            1.0,
            false,
            phase == 1,
            false,
            false,
            phase == 3,
            0.0,
            None,
            3.0,
        );
        if clipped {
            assert!(visuals.update_stencil_ref(handle, 1));
        }
    }

    visuals
}

fn median(values: &mut [u64]) -> u64 {
    values.sort_unstable();
    values[values.len() / 2]
}

fn stream_totals(visuals: &VisualWorld) -> (usize, usize, usize) {
    let streams = [
        visuals.opaque_stream(),
        visuals.cutout_stream(),
        visuals.transparent_single_stream(),
        visuals.overlay_stream(),
    ];
    let operation_count = streams.iter().map(|(ops, _)| ops.len()).sum::<usize>();
    let instance_count = streams
        .iter()
        .map(|(_, instances)| instances.len())
        .sum::<usize>();

    // Before 5fc985b, ordinary render views cloned these three stream pairs.
    // Current code borrows them. Transparent-single already borrowed its stream.
    let ordinary_view_payload_bytes = [
        visuals.opaque_stream(),
        visuals.cutout_stream(),
        visuals.overlay_stream(),
    ]
    .iter()
    .map(|(ops, instances)| std::mem::size_of_val(*ops) + std::mem::size_of_val(*instances))
    .sum();

    (operation_count, instance_count, ordinary_view_payload_bytes)
}

fn profile_scenario(name: &str, clipped: bool, instance_count: usize, iterations: usize) {
    let mut elapsed_ns = Vec::with_capacity(iterations);
    let mut allocation_calls = Vec::with_capacity(iterations);
    let mut allocated_bytes = Vec::with_capacity(iterations);
    let mut final_world = None;

    for _ in 0..iterations {
        let mut visuals = build_workload(instance_count, clipped);

        ALLOCATION_CALLS.store(0, Ordering::Relaxed);
        ALLOCATED_BYTES.store(0, Ordering::Relaxed);
        COUNTING.store(true, Ordering::SeqCst);
        let started = Instant::now();
        assert!(visuals.prepare_draw_cache());
        let elapsed = started.elapsed();
        COUNTING.store(false, Ordering::SeqCst);

        elapsed_ns.push(elapsed.as_nanos() as u64);
        allocation_calls.push(ALLOCATION_CALLS.load(Ordering::Relaxed));
        allocated_bytes.push(ALLOCATED_BYTES.load(Ordering::Relaxed));
        final_world = Some(visuals);
    }

    let visuals = final_world.expect("at least one profiling iteration");
    let (operation_count, stream_instance_count, ordinary_view_payload_bytes) =
        stream_totals(&visuals);

    println!(
        "RENDER_STREAM_PROFILE scenario={name} workload_instances={instance_count} \
         iterations={iterations} cache_time_ns_median={} cache_alloc_calls_median={} \
         cache_alloc_bytes_median={} stream_operations={} stream_instances={} \
         ordinary_view_stream_payload_bytes={ordinary_view_payload_bytes}",
        median(&mut elapsed_ns),
        median(&mut allocation_calls),
        median(&mut allocated_bytes),
        operation_count,
        stream_instance_count,
    );
}

#[test]
#[ignore = "manual release-mode revision comparison"]
fn renderer_optimization_profile() {
    let instance_count = std::env::var("MITTENS_RENDER_PROFILE_INSTANCES")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(2048);
    let iterations = std::env::var("MITTENS_RENDER_PROFILE_ITERATIONS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(25);
    assert!(instance_count > 0);
    assert!(iterations > 0);

    profile_scenario("unclipped", false, instance_count, iterations);
    profile_scenario("clipped", true, instance_count, iterations);
}
