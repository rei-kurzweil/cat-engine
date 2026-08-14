use mittens_engine::engine::ecs::SignalEmitter;
use mittens_engine::{engine, scripting, utils};

fn main() {
    utils::logger::init();

    let world = engine::ecs::World::default();
    let mut universe = engine::Universe::new(world);
    let output = scripting::MeowMeowRunner::eval_with_runtime_spec(
        include_str!("runtime-spec-smoke.mms"),
        &mut universe.world,
        &mut universe.systems.rx,
        Some(&mut universe.render_assets),
        &mut universe.command_queue,
    );

    assert!(
        output.errors.is_empty(),
        "RuntimeSpec evaluation failed: {:?}",
        output.errors
    );
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

    engine::Windowing::run_app(universe).expect("Windowing failed");
}
