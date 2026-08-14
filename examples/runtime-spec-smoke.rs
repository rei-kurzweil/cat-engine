use mittens_engine::engine::ecs::{CommandQueue, RxWorld, World};
use mittens_engine::scripting::MeowMeowRunner;

fn main() {
    let mut world = World::default();
    let mut rx = RxWorld::default();
    let mut emit = CommandQueue::new();

    let output = MeowMeowRunner::eval_with_runtime_spec(
        include_str!("runtime-spec-smoke.mms"),
        &mut world,
        &mut rx,
        None,
        &mut emit,
    );
    assert!(output.errors.is_empty(), "{}", output.errors.join("\n"));

    let normalized_names: Vec<_> = world
        .all_components()
        .filter_map(|id| world.component_name(id))
        .map(|name| name.replace('_', "").to_lowercase())
        .collect();
    assert_eq!(
        normalized_names
            .iter()
            .filter(|name| *name == "camera3d")
            .count(),
        1
    );
    assert_eq!(
        normalized_names
            .iter()
            .filter(|name| *name == "renderable")
            .count(),
        3
    );
    assert_eq!(
        normalized_names
            .iter()
            .filter(|name| *name == "emissive")
            .count(),
        3
    );

    println!("RuntimeSpec smoke passed: camera + three emissive cubes");
}
