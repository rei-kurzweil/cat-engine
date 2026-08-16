use mittens_engine::engine::ecs::SignalEmitter;
use mittens_engine::{engine, scripting, utils};

fn main() {
    utils::logger::init();

    let world = engine::ecs::World::default();
    let mut universe = engine::Universe::new(world);
    let (mut script_session, intents) = scripting::RuntimeSpecSession::start(
        include_str!("runtime-spec-smoke.mms"),
        &mut universe.world,
        &mut universe.systems.rx,
        Some(&mut universe.render_assets),
        &mut universe.command_queue,
    )
    .expect("RuntimeSpec evaluation failed");
    for intent in intents {
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

    engine::Windowing::run_app_with_frame_hook(universe, move |universe| {
        let output = script_session.service_callbacks(
            &mut universe.world,
            &mut universe.systems.rx,
            Some(&mut universe.render_assets),
            &mut universe.command_queue,
        );
        for error in output.errors {
            eprintln!("[mms] callback error: {error}");
        }
        for intent in output.intents {
            universe
                .command_queue
                .push_intent_now(engine::ecs::ComponentId::default(), intent);
        }
    })
    .expect("Windowing failed");
}
