use mittens_engine::engine::ecs::SignalEmitter;
use mittens_engine::engine::{self, ecs::World};
use mittens_engine::scripting::MeowMeowRunner;

fn main() {
    let world = World::default();
    let mut universe = engine::Universe::new(world);

    let output = MeowMeowRunner::eval_with_runtime_spec(
        include_str!("runtime-spec-smoke.mms"),
        &mut universe.world,
        &mut universe.systems.rx,
        Some(&mut universe.render_assets),
        &mut universe.command_queue,
    );
    assert!(output.errors.is_empty(), "{}", output.errors.join("\n"));
    for intent in output.intents {
        universe
            .command_queue
            .push_intent_now(engine::ecs::ComponentId::default(), intent);
    }
    universe.systems.process_commands(
        &mut universe.world,
        &mut universe.visuals,
        &mut universe.render_assets,
        &mut universe.command_queue,
    );

    let normalized_names: Vec<_> = universe
        .world
        .all_components()
        .filter_map(|id| universe.world.component_name(id))
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
            .filter(|name| *name == "directionallight")
            .count(),
        1
    );
    assert_eq!(
        normalized_names
            .iter()
            .filter(|name| *name == "emissive")
            .count(),
        3
    );
    let post_processing = universe.visuals.post_processing();
    assert!(post_processing.is_active());
    let bloom = post_processing.bloom.as_ref().expect("bloom is configured");
    assert!((bloom.intensity - 1.2).abs() < f32::EPSILON);

    println!(
        "RuntimeSpec smoke passed: configured camera + light + three emissive cubes + active bloom"
    );
}
