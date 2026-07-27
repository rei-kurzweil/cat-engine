macro_rules! mms_live_example {
    ($scene:literal) => {
        fn main() {
            use mittens_engine::engine::ecs::SignalEmitter;

            mittens_engine::example_support::ensure_model_assets();
            mittens_engine::utils::logger::init();

            let world = mittens_engine::engine::ecs::World::default();
            let mut universe = mittens_engine::engine::Universe::new(world);
            let output = mittens_engine::scripting::MeowMeowRunner::eval_with_world_and_assets_at_path(
                include_str!($scene),
                Some(concat!("examples/", $scene)),
                &mut universe.world,
                &mut universe.systems.rx,
                Some(&mut universe.render_assets),
                &mut universe.command_queue,
            );

            for error in &output.errors {
                eprintln!("[mms] {error}");
            }
            if !output.errors.is_empty() {
                std::process::exit(1);
            }
            for intent in output.intents {
                universe.command_queue.push_intent_now(
                    mittens_engine::engine::ecs::ComponentId::default(),
                    intent,
                );
            }
            universe.systems.process_commands(
                &mut universe.world,
                &mut universe.visuals,
                &mut universe.render_assets,
                &mut universe.command_queue,
            );
            universe.enable_repl();
            mittens_engine::engine::Windowing::run_app(universe).expect("Windowing failed");
        }
    };
}
