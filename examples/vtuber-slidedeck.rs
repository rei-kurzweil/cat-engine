use mittens_engine::engine::ecs::component::{ControllerHand, ControllerXRComponent};
use mittens_engine::engine::ecs::{
    ComponentId, EventSignal, Signal, SignalEmitter, SignalKind, World,
};
use mittens_engine::{engine, scripting, utils};

#[path = "example_util/mod.rs"]
mod example_util;

fn component_label(world: &World, id: ComponentId) -> String {
    world.component_name(id).unwrap_or("?").to_string()
}

fn controller_hand_for_component(world: &World, start: ComponentId) -> Option<ControllerHand> {
    let mut current = start;
    loop {
        if let Some(controller) = world.get_component_by_id_as::<ControllerXRComponent>(current) {
            return Some(controller.hand);
        }
        current = world.parent_of(current)?;
    }
}

fn on_xr_pointer_event(world: &mut World, _emit: &mut dyn SignalEmitter, signal: &Signal) {
    let Some(event) = signal.event.as_ref() else {
        return;
    };
    let (kind, raycaster, renderable) = match event {
        EventSignal::DragStart {
            raycaster,
            renderable,
            ..
        } => ("DragStart", *raycaster, *renderable),
        EventSignal::DragMove {
            raycaster,
            renderable,
            ..
        } => ("DragMove", *raycaster, *renderable),
        EventSignal::DragEnd {
            raycaster,
            renderable,
            ..
        } => ("DragEnd", *raycaster, *renderable),
        EventSignal::Click {
            raycaster,
            renderable,
            ..
        } => ("Click", *raycaster, *renderable),
        _ => return,
    };

    let Some(hand) = controller_hand_for_component(world, raycaster) else {
        return;
    };
    println!(
        "[vtuber-slidedeck] hand={hand:?} kind={kind} raycaster={} renderable={}",
        component_label(world, raycaster),
        component_label(world, renderable),
    );
}

fn on_xr_button_event(world: &mut World, _emit: &mut dyn SignalEmitter, signal: &Signal) {
    let Some(event) = signal.event.as_ref() else {
        return;
    };
    let (edge, source, hand, control, value) = match event {
        EventSignal::XrButtonDown {
            source_component,
            hand,
            control,
            value,
        } => ("down", *source_component, *hand, *control, *value),
        EventSignal::XrButtonUp {
            source_component,
            hand,
            control,
            value,
        } => ("up", *source_component, *hand, *control, *value),
        _ => return,
    };

    println!(
        "[vtuber-slidedeck][xr-button] edge={edge} hand={hand:?} control={control:?} value={value:.3} source={} ({source:?})",
        component_label(world, source),
    );
}

#[cfg(test)]
mod tests {
    use mittens_engine::scripting;

    #[test]
    fn mms_scene_evaluates_with_manual_step_handlers() {
        let output = scripting::MeowMeowRunner::eval_with_path(
            include_str!("vtuber-slidedeck.mms"),
            "examples/vtuber-slidedeck.mms",
        );
        assert!(output.errors.is_empty(), "{:?}", output.errors);
    }
}

fn main() {
    mittens_engine::example_support::ensure_model_assets();
    utils::logger::init();

    let output = scripting::MeowMeowRunner::eval_with_path(
        include_str!("vtuber-slidedeck.mms"),
        "examples/vtuber-slidedeck.mms",
    );

    for error in &output.errors {
        eprintln!("[mms] {error}");
    }
    assert!(
        output.errors.is_empty(),
        "MMS evaluation produced errors: {:?}",
        output.errors,
    );
    println!(
        "[mms] {} intent(s) from vtuber-slidedeck.mms",
        output.intents.len()
    );
    println!("[vtuber-slidedeck] controls: B = next slide, A = previous slide");

    let world = engine::ecs::World::default();
    let mut universe = engine::Universe::new(world);

    let scope = engine::ecs::ComponentId::default();
    for intent in output.intents {
        universe.command_queue.push_intent_now(scope, intent);
    }

    universe.systems.process_commands(
        &mut universe.world,
        &mut universe.visuals,
        &mut universe.render_assets,
        &mut universe.command_queue,
    );

    let background_root = universe.world.add_component(
        engine::ecs::component::BackgroundComponent::new().with_occlusion_and_lighting(),
    );
    universe.add(background_root);

    let cloud_params = example_util::CloudRingParams {
        cloud_count: 10,
        radius: 34.0,
        center_y: 8.5,
        puffs_per_cloud: 28,
        angle_jitter: 0.30,
        high_y_probability: 0.45,
        high_y_multiplier: 1.28,
        seed: 0x51_1D_ED_ECu32,
    };
    example_util::spawn_cloud_ring(&mut universe, background_root, cloud_params);

    for kind in [
        SignalKind::DragStart,
        SignalKind::DragMove,
        SignalKind::DragEnd,
        SignalKind::Click,
    ] {
        universe
            .systems
            .rx
            .add_global_handler(kind, on_xr_pointer_event);
    }
    for kind in [SignalKind::XrButtonDown, SignalKind::XrButtonUp] {
        universe
            .systems
            .rx
            .add_global_handler(kind, on_xr_button_event);
    }

    universe.enable_repl();
    engine::Windowing::run_app(universe).expect("Windowing failed");
}
