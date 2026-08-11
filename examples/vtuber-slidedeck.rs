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
    use mittens_engine::engine;
    use mittens_engine::engine::ecs::component::{
        ControllerHand, InputXRGamepadComponent, TextComponent, TransformComponent, XrButtonControl,
    };
    use mittens_engine::engine::ecs::system::TransformSystem;
    use mittens_engine::engine::ecs::{EventSignal, IntentValue, Signal, SignalEmitter};
    use mittens_engine::scripting;

    #[test]
    fn button_b_places_a_detached_slide_and_advances_its_content() {
        let world = engine::ecs::World::default();
        let mut universe = engine::Universe::new(world);
        let output = scripting::MeowMeowRunner::eval_with_world_and_assets_at_path(
            include_str!("vtuber-slidedeck.mms"),
            Some("examples/vtuber-slidedeck.mms"),
            &mut universe.world,
            &mut universe.systems.rx,
            Some(&mut universe.render_assets),
            &mut universe.command_queue,
        );
        assert!(output.errors.is_empty(), "{:?}", output.errors);

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

        let controls = universe
            .world
            .all_components()
            .find(|id| {
                universe
                    .world
                    .get_component_by_id_as::<InputXRGamepadComponent>(*id)
                    .is_some()
            })
            .expect("XR gamepad component");
        universe.systems.rx.dispatch_event_handlers(
            &mut universe.world,
            &Signal::event(
                controls,
                EventSignal::XrButtonDown {
                    source_component: controls,
                    hand: ControllerHand::Right,
                    control: XrButtonControl::ButtonB,
                    value: 1.0,
                },
            ),
        );
        universe.systems.process_commands(
            &mut universe.world,
            &mut universe.visuals,
            &mut universe.render_assets,
            &mut universe.command_queue,
        );
        universe.systems.animation.tick_with_beat(
            &mut universe.world,
            0.0,
            60.0,
            &mut universe.systems.rx,
        );
        universe.systems.process_commands(
            &mut universe.world,
            &mut universe.visuals,
            &mut universe.render_assets,
            &mut universe.command_queue,
        );

        let texts = universe
            .world
            .all_components()
            .filter_map(|id| {
                universe
                    .world
                    .get_component_by_id_as::<TextComponent>(id)
                    .map(|text| text.text.clone())
            })
            .collect::<Vec<_>>();
        assert!(
            texts.iter().any(|text| {
                text == "short form video creators hate it when you use this one simple trick!"
            }),
            "live text values after ButtonB: {texts:?}",
        );

        let named_transform = |name: &str| {
            universe
                .world
                .all_components()
                .find(|id| {
                    universe
                        .world
                        .get_component_record(*id)
                        .is_some_and(|record| record.name == name)
                        && universe
                            .world
                            .get_component_by_id_as::<TransformComponent>(*id)
                            .is_some()
                })
                .unwrap_or_else(|| panic!("transform named {name}"))
        };
        let anchor = named_transform("xr_camera_wrapper");
        let slide_root = named_transform("detached_slide_root");
        let locomotion_root = named_transform("avatar_locomotion_root");

        fn assert_matrix_near(actual: [[f32; 4]; 4], expected: [[f32; 4]; 4]) {
            for (actual, expected) in actual
                .into_iter()
                .flatten()
                .zip(expected.into_iter().flatten())
            {
                assert!((actual - expected).abs() < 1e-4, "{actual} != {expected}");
            }
        }

        let anchor_at_snapshot = TransformSystem::world_model(&universe.world, anchor).unwrap();
        let slide_at_snapshot = TransformSystem::world_model(&universe.world, slide_root).unwrap();
        assert_matrix_near(slide_at_snapshot, anchor_at_snapshot);

        universe.command_queue.push_intent_now(
            locomotion_root,
            IntentValue::UpdateTransform {
                component_id: locomotion_root,
                translation: [2.0, 0.0, 0.0],
                rotation_quat_xyzw: [0.0, 0.0, 0.0, 1.0],
                scale: [1.0, 1.0, 1.0],
            },
        );
        universe.systems.process_commands(
            &mut universe.world,
            &mut universe.visuals,
            &mut universe.render_assets,
            &mut universe.command_queue,
        );

        let moved_anchor = TransformSystem::world_model(&universe.world, anchor).unwrap();
        let still_detached_slide =
            TransformSystem::world_model(&universe.world, slide_root).unwrap();
        assert_ne!(moved_anchor[3], anchor_at_snapshot[3]);
        assert_matrix_near(still_detached_slide, slide_at_snapshot);

        universe.systems.rx.dispatch_event_handlers(
            &mut universe.world,
            &Signal::event(
                controls,
                EventSignal::XrButtonDown {
                    source_component: controls,
                    hand: ControllerHand::Right,
                    control: XrButtonControl::ButtonB,
                    value: 1.0,
                },
            ),
        );
        universe.systems.process_commands(
            &mut universe.world,
            &mut universe.visuals,
            &mut universe.render_assets,
            &mut universe.command_queue,
        );
        assert_matrix_near(
            TransformSystem::world_model(&universe.world, slide_root).unwrap(),
            moved_anchor,
        );
    }
}

fn main() {
    mittens_engine::example_support::ensure_model_assets();
    utils::logger::init();

    let world = engine::ecs::World::default();
    let mut universe = engine::Universe::new(world);
    let output = scripting::MeowMeowRunner::eval_with_world_and_assets_at_path(
        include_str!("vtuber-slidedeck.mms"),
        Some("examples/vtuber-slidedeck.mms"),
        &mut universe.world,
        &mut universe.systems.rx,
        Some(&mut universe.render_assets),
        &mut universe.command_queue,
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
