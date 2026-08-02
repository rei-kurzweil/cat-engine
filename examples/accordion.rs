use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use mittens_engine::engine::ecs::component::{
    ColorComponent, Display, EdgeInsets, SizeDimension, StyleComponent, TextComponent,
    TransformComponent,
};
use mittens_engine::engine::ecs::{
    ComponentId, EventSignal, IntentValue, SignalEmitter, SignalKind, World,
};
use mittens_engine::{engine, scripting, utils};

fn find_named(world: &World, name: &str) -> Option<ComponentId> {
    world
        .all_components()
        .find(|&id| world.component_label(id) == Some(name))
}

fn spawn_native_accordion_body(world: &mut World, generation: usize) -> ComponentId {
    let body =
        world.add_component_boxed_named("accordion_body", Box::new(TransformComponent::new()));
    let body_style = world.add_component_boxed_named(
        "native_accordion_body_style",
        Box::new({
            let mut style = StyleComponent::new();
            style.display = Some(Display::Block);
            style.width = SizeDimension::Percent(100.0);
            style.margin = EdgeInsets::axes(0.0, 0.3);
            style.padding = EdgeInsets::axes(1.0, 1.0);
            style
        }),
    );

    let card = world
        .add_component_boxed_named("native_restored_card", Box::new(TransformComponent::new()));
    let card_style = world.add_component_boxed_named(
        "native_restored_card_style",
        Box::new({
            let mut style = StyleComponent::new();
            style.display = Some(Display::Block);
            style.width = SizeDimension::Percent(100.0);
            style.height = SizeDimension::GlyphUnits(9.0);
            style.background_color = Some([0.055, 0.035, 0.20, 0.97]);
            style.background_z = Some(-0.01);
            style.color = Some([0.94, 0.90, 1.0, 1.0]);
            style
        }),
    );

    let text_root = world.add_component_boxed_named(
        "native_restored_text_root",
        Box::new(TransformComponent::new().with_position(0.0, 0.0, 0.02)),
    );
    let text = world.add_component_boxed_named(
        "native_restored_text",
        Box::new(TextComponent::new(format!(
            "native Rust responder rebuilt generation {generation}\nDataEvent payload supplied this body mount"
        ))),
    );
    let text_color = world.add_component_boxed_named(
        "native_restored_text_color",
        Box::new(ColorComponent::rgba(0.78, 0.72, 1.0, 1.0)),
    );

    let _ = world.add_child(body, body_style);
    let _ = world.add_child(body, card);
    let _ = world.add_child(card, card_style);
    let _ = world.add_child(card, text_root);
    let _ = world.add_child(text_root, text);
    let _ = world.add_child(text, text_color);
    body
}

fn main() {
    utils::logger::init();

    let world = World::default();
    let mut universe = engine::Universe::new(world);
    let output = scripting::MeowMeowRunner::eval_with_world_and_assets_at_path(
        include_str!("accordion.mms"),
        Some("examples/accordion.mms"),
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

    let native_panel = find_named(&universe.world, "native_accordion")
        .expect("accordion.mms must create #native_accordion");
    let native_status = find_named(&universe.world, "native_accordion_status")
        .expect("accordion.mms must create #native_accordion_status");
    let generation = Arc::new(AtomicUsize::new(0));
    let handler_generation = Arc::clone(&generation);

    universe.systems.rx.add_handler_closure(
        SignalKind::DataEvent,
        native_panel,
        move |world, emit, signal| {
            let Some(EventSignal::DataEvent { name, payload }) = signal.event.as_ref() else {
                return;
            };

            match name.as_str() {
                "AccordionMinimized" => {
                    emit.push_intent_now(
                        native_status,
                        IntentValue::SetText {
                            component_id: native_status,
                            text: "Native panel: body removed".to_string(),
                        },
                    );
                }
                "AccordionRestoreRequested" => {
                    let Some(body_mount) = *payload else {
                        eprintln!(
                            "[accordion-example] restore request omitted accordion_body_mount"
                        );
                        return;
                    };
                    let next_generation = handler_generation.fetch_add(1, Ordering::Relaxed) + 1;
                    let body = spawn_native_accordion_body(world, next_generation);
                    world.init_component_tree(body, emit);
                    emit.push_intent_now(
                        body,
                        IntentValue::Attach {
                            parent: body_mount,
                            child: body,
                        },
                    );
                    emit.push_intent_now(
                        native_status,
                        IntentValue::SetText {
                            component_id: native_status,
                            text: format!("Native panel: restored generation {next_generation}"),
                        },
                    );
                }
                _ => {}
            }
        },
    );

    let scope = ComponentId::default();
    for intent in output.intents {
        universe.command_queue.push_intent_now(scope, intent);
    }
    universe.systems.process_commands(
        &mut universe.world,
        &mut universe.visuals,
        &mut universe.render_assets,
        &mut universe.command_queue,
    );

    universe.enable_repl();
    engine::Windowing::run_app(universe).expect("Windowing failed");
}
