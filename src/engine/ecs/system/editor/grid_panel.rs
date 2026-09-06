use std::sync::{Arc, LazyLock, Mutex};

use crate::engine::ecs::component::{
    ColorComponent, DataComponent, DataValue, Display, EdgeInsets, EditorComponent,
    GridVisualSpace, OptionComponent, RaycastableComponent, SelectionEntry, SizeDimension,
    StyleComponent, TextAlign, TextComponent, TransformComponent, TransformGizmoComponent,
    style::VerticalAlign,
};
use crate::engine::ecs::system::GridSystem;
use crate::engine::ecs::system::data_renderer_system::{
    DataRendererSystem, ItemRendererSpec, RendererSpec, UiItem, UiItemKind,
};
use crate::engine::ecs::system::editor::context::{
    EditorContextState, apply_editor_root_selection,
};
use crate::engine::ecs::system::editor::world_panel::{
    PANEL_CONTENT_SLOT_SELECTOR, world_panel_item_label,
};
use crate::engine::ecs::system::grid_system::GridSpawnSpec;
use crate::engine::ecs::system::panel_system::{
    PanelActionKind, PanelKind, build_editor_panel_component_expr, decode_panel_action_payload,
    icons_asset_path, is_descendant_or_self,
};
use crate::engine::ecs::system::selection_system::apply_selection_set;
use crate::engine::ecs::{ComponentId, EventSignal, SignalEmitter, World};
use crate::scripting::component_registry::spawn_tree;

pub(crate) const GRID_PANEL_ROOT_SELECTOR: &str = "#grid_panel_root";
pub(crate) const GRID_PANEL_SELECTION_SELECTOR: &str = "#grid_panel_selection";
pub(crate) const GRID_PANEL_ADD_BUTTON_SELECTOR: &str = "#grid_add_button";
pub(crate) const GRID_PANEL_ITEM_PREFIX: &str = "grid_item_";
pub(crate) const GRID_PANEL_ROW_PAYLOAD_NAME: &str = "grid_panel_row_payload";
pub(crate) const GRID_PANEL_VISIBILITY_PAYLOAD_NAME: &str = "grid_panel_visibility_payload";
pub(crate) const GRID_PANEL_ENABLED_PAYLOAD_NAME: &str = "grid_panel_enabled_payload";
pub(crate) const GRID_PANEL_BINDING_PAYLOAD_NAME: &str = "grid_panel_binding_payload";
pub(crate) const GRID_PANEL_DELETE_PAYLOAD_NAME: &str = "grid_panel_delete_payload";
pub(crate) const GRID_PANEL_VISUAL_SPACE_PAYLOAD_NAME: &str = "grid_panel_visual_space_payload";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct GridPanelState {
    pub(crate) active_editor: Option<ComponentId>,
    pub(crate) selected_grid_transform: Option<ComponentId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum GridPanelEvent {
    SelectionChanged {
        editor_root: Option<ComponentId>,
        selected_component: Option<ComponentId>,
    },
    GridDeleted {
        owner_transform: ComponentId,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GridPanelEntry {
    pub(crate) grid_component: ComponentId,
    pub(crate) owner_transform: ComponentId,
    pub(crate) label: String,
    pub(crate) shown: bool,
    pub(crate) enabled: bool,
    pub(crate) selected: bool,
    pub(crate) bound: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GridPanelModel {
    pub(crate) title: String,
    pub(crate) rows: Vec<GridPanelEntry>,
    pub(crate) active_editor: Option<ComponentId>,
}

pub(crate) fn reduce_grid_panel_state(
    old: &GridPanelState,
    event: &GridPanelEvent,
) -> GridPanelState {
    let mut new = old.clone();
    match event {
        GridPanelEvent::SelectionChanged {
            editor_root,
            selected_component,
        } => {
            new.active_editor = *editor_root;
            new.selected_grid_transform = *selected_component;
        }
        GridPanelEvent::GridDeleted { owner_transform } => {
            if new.selected_grid_transform == Some(*owner_transform) {
                new.selected_grid_transform = None;
            }
        }
    }
    new
}

pub(crate) fn build_grid_panel_model(
    world: &World,
    grids: &GridSystem,
    editor_root: ComponentId,
    active_grid_owner_transform: Option<ComponentId>,
) -> GridPanelModel {
    let manipulation_target = selected_manipulation_target(world, editor_root);
    let rows = grids
        .enumerate_grids_for_editor(world, editor_root)
        .into_iter()
        .map(|entry| GridPanelEntry {
            grid_component: entry.grid_component,
            owner_transform: entry.owner_transform,
            label: world_panel_item_label(world, entry.owner_transform),
            shown: !entry.hidden,
            enabled: entry.enabled,
            selected: active_grid_owner_transform == Some(entry.owner_transform),
            bound: manipulation_target
                .and_then(|target| GridSystem::bound_grid_transform(world, target))
                == Some(entry.owner_transform),
        })
        .collect();

    GridPanelModel {
        title: "Grids".to_string(),
        rows,
        active_editor: Some(editor_root),
    }
}

fn selected_manipulation_target(world: &World, editor_root: ComponentId) -> Option<ComponentId> {
    let selected = world
        .get_component_by_id_as::<EditorComponent>(editor_root)?
        .selected?;
    world
        .children_of(selected)
        .iter()
        .find_map(|&child| {
            world
                .get_component_by_id_as::<TransformGizmoComponent>(child)
                .and_then(|gizmo| gizmo.target_transform)
        })
        .or(Some(selected))
}

pub(crate) fn grid_panel_items(model: &GridPanelModel) -> Vec<UiItem> {
    model
        .rows
        .iter()
        .enumerate()
        .map(|(index, row)| UiItem {
            key: format!("{GRID_PANEL_ITEM_PREFIX}{index}"),
            kind: UiItemKind::Component,
            label: row.label.clone(),
            selected: row.selected,
            target_ref: Some(row.owner_transform),
        })
        .collect()
}

fn grid_panel_row_render_fn(
    world: &mut World,
    emit: &mut dyn SignalEmitter,
    item: &UiItem,
) -> Result<ComponentId, String> {
    let owner_transform = item
        .target_ref
        .ok_or_else(|| "grid row missing owner transform".to_string())?;
    let grids = GridSystem::new();
    let Some(entry) = grids.grid_owned_by_transform(world, owner_transform) else {
        return Err("grid row missing grid entry".to_string());
    };
    let (shown, enabled) = (!entry.hidden, entry.enabled);
    let visual_space = world
        .get_component_by_id_as::<crate::engine::ecs::component::GridComponent>(
            entry.grid_component,
        )
        .map(|grid| grid.visual_space)
        .unwrap_or(GridVisualSpace::Local);
    let bound = selected_manipulation_target(world, entry.editor_root)
        .and_then(|target| GridSystem::bound_grid_transform(world, target))
        == Some(owner_transform);
    Ok(spawn_grid_panel_row_tree(
        world,
        emit,
        &item.key,
        entry.grid_component,
        owner_transform,
        &item.label,
        item.selected,
        shown,
        enabled,
        bound,
        visual_space,
    ))
}

pub(crate) static GRID_PANEL_ROW_SPEC: LazyLock<ItemRendererSpec> =
    LazyLock::new(|| RendererSpec::Rust {
        render_fn: Box::new(grid_panel_row_render_fn),
    });

fn spawn_grid_panel_row_tree(
    world: &mut World,
    emit: &mut dyn SignalEmitter,
    row_name: &str,
    grid_component: ComponentId,
    owner_transform: ComponentId,
    label: &str,
    selected: bool,
    shown: bool,
    enabled: bool,
    bound: bool,
    visual_space: GridVisualSpace,
) -> ComponentId {
    let row_root = world.add_component_boxed_named(row_name, Box::new(TransformComponent::new()));
    let row_option = world.add_component_boxed_named(
        format!("{row_name}_option"),
        Box::new(OptionComponent::new()),
    );
    let row_raycastable = world.add_component_boxed_named(
        format!("{row_name}_raycastable"),
        Box::new(RaycastableComponent::click_only()),
    );
    let row_payload = world.add_component_boxed_named(
        GRID_PANEL_ROW_PAYLOAD_NAME,
        Box::new(
            DataComponent::new()
                .with_entry("row_name", DataValue::Text(row_name.to_string()))
                .with_entry("label", DataValue::Text(label.to_string()))
                .with_entry("target_component", DataValue::Component(owner_transform))
                .with_entry("owner_transform", DataValue::Component(owner_transform))
                .with_entry("grid_component", DataValue::Component(grid_component)),
        ),
    );
    let _ = world.add_child(row_root, row_option);
    let _ = world.add_child(row_root, row_raycastable);
    let _ = world.add_child(row_root, row_payload);

    let style = world.add_component_boxed_named(
        format!("{row_name}_style"),
        Box::new({
            let mut style = StyleComponent::new();
            style.display = Some(Display::Block);
            style.width = SizeDimension::Percent(100.0);
            style.margin = EdgeInsets::axes(0.25, 0.20);
            style.padding = EdgeInsets::axes(0.30, 0.35);
            style.background_color = Some(if selected {
                [1.00, 0.88, 0.20, 0.96]
            } else {
                [0.92, 0.97, 0.92, 1.0]
            });
            style.background_z = Some(0.001);
            style.overflow = crate::engine::ecs::component::Overflow::Visible;
            style
        }),
    );
    let _ = world.add_child(row_root, style);

    let body = world.add_component_boxed_named(
        format!("{row_name}_body"),
        Box::new(TransformComponent::new()),
    );
    let body_style = world.add_component_boxed_named(
        format!("{row_name}_body_style"),
        Box::new({
            let mut style = StyleComponent::new();
            style.display = Some(Display::InlineBlock);
            style.width = SizeDimension::GlyphUnits(13.0);
            style.height = SizeDimension::GlyphUnits(2.3);
            style.padding = EdgeInsets::axes(0.15, 0.10);
            style.vertical_align = VerticalAlign::Middle;
            style.color = Some(if selected {
                [0.08, 0.08, 0.02, 1.0]
            } else {
                [0.06, 0.09, 0.08, 1.0]
            });
            style
        }),
    );
    let body_text_root = world.add_component_boxed_named(
        format!("{row_name}_body_text_root"),
        Box::new(TransformComponent::new().with_position(0.0, 0.0, 0.005)),
    );
    let body_text = world.add_component_boxed_named(
        format!("{row_name}_body_text"),
        Box::new(TextComponent::new(label.to_string())),
    );
    let body_text_color = world.add_component_boxed_named(
        format!("{row_name}_body_text_color"),
        Box::new(ColorComponent::rgba(
            if selected { 0.08 } else { 0.06 },
            if selected { 0.08 } else { 0.09 },
            if selected { 0.02 } else { 0.08 },
            1.0,
        )),
    );
    let _ = world.add_child(row_root, body);
    let _ = world.add_child(body, body_style);
    let _ = world.add_child(body, body_text_root);
    let _ = world.add_child(body_text_root, body_text);
    let _ = world.add_child(body_text, body_text_color);

    let visibility = spawn_grid_icon_button(
        world,
        emit,
        row_name,
        "visibility",
        GRID_PANEL_VISIBILITY_PAYLOAD_NAME,
        owner_transform,
        row_name,
        if shown { "Visible" } else { "Hidden" },
        if shown {
            [0.28, 0.29, 0.31, 1.0]
        } else {
            [0.16, 0.17, 0.18, 1.0]
        },
        None,
        None,
    );
    let enabled_toggle = spawn_grid_icon_button(
        world,
        emit,
        row_name,
        "enabled",
        GRID_PANEL_ENABLED_PAYLOAD_NAME,
        owner_transform,
        row_name,
        if enabled { "On" } else { "Off" },
        if enabled {
            [0.12, 0.36, 0.72, 1.0]
        } else {
            [0.45, 0.30, 0.08, 1.0]
        },
        None,
        Some(3.5),
    );
    let binding = spawn_grid_icon_button(
        world,
        emit,
        row_name,
        "binding",
        GRID_PANEL_BINDING_PAYLOAD_NAME,
        owner_transform,
        row_name,
        if bound { "Unbind" } else { "Bind" },
        if bound {
            [0.58, 0.18, 0.58, 1.0]
        } else {
            [0.20, 0.42, 0.36, 1.0]
        },
        None,
        Some(4.5),
    );
    let delete = spawn_grid_icon_button(
        world,
        emit,
        row_name,
        "delete",
        GRID_PANEL_DELETE_PAYLOAD_NAME,
        owner_transform,
        row_name,
        "",
        [0.72, 0.15, 0.15, 1.0],
        Some("delete_x_icon"),
        Some(3.5),
    );
    let visual_space_toggle = spawn_grid_icon_button(
        world,
        emit,
        row_name,
        "visual_space",
        GRID_PANEL_VISUAL_SPACE_PAYLOAD_NAME,
        owner_transform,
        row_name,
        match visual_space {
            GridVisualSpace::Local => "Local",
            GridVisualSpace::World => "World",
        },
        [0.24, 0.38, 0.52, 1.0],
        None,
        Some(5.0),
    );
    let _ = world.add_child(row_root, visibility);
    let _ = world.add_child(row_root, enabled_toggle);
    let _ = world.add_child(row_root, binding);
    let _ = world.add_child(row_root, delete);
    let _ = world.add_child(row_root, visual_space_toggle);

    row_root
}

fn spawn_grid_icon_button(
    world: &mut World,
    emit: &mut dyn SignalEmitter,
    row_name: &str,
    suffix: &str,
    payload_name: &str,
    owner_transform: ComponentId,
    item_key: &str,
    text: &str,
    background_color: [f32; 4],
    icon_name: Option<&str>,
    width_glyph_units: Option<f32>,
) -> ComponentId {
    let root = world.add_component_boxed_named(
        format!("{row_name}_{suffix}_button"),
        Box::new(TransformComponent::new()),
    );
    let option = world.add_component_boxed_named(
        format!("{row_name}_{suffix}_option"),
        Box::new(OptionComponent::new()),
    );
    let raycastable = world.add_component_boxed_named(
        format!("{row_name}_{suffix}_raycastable"),
        Box::new(RaycastableComponent::click_only()),
    );
    let payload = world.add_component_boxed_named(
        payload_name,
        Box::new(
            DataComponent::new()
                .with_entry("row_name", DataValue::Text(item_key.to_string()))
                .with_entry("target_component", DataValue::Component(owner_transform))
                .with_entry("label", DataValue::Text(text.to_string())),
        ),
    );
    let style = world.add_component_boxed_named(
        format!("{row_name}_{suffix}_style"),
        Box::new({
            let mut style = StyleComponent::new();
            style.display = Some(Display::InlineBlock);
            if let Some(w) = width_glyph_units {
                style.width = SizeDimension::GlyphUnits(w);
            }
            style.height = SizeDimension::GlyphUnits(2.3);
            style.margin = EdgeInsets {
                left: SizeDimension::GlyphUnits(0.20),
                right: SizeDimension::GlyphUnits(0.0),
                top: SizeDimension::GlyphUnits(0.0),
                bottom: SizeDimension::GlyphUnits(0.0),
            };
            style.text_align = TextAlign::Center;
            style.vertical_align = VerticalAlign::Middle;
            style.background_color = Some(background_color);
            style.color = Some([0.96, 0.98, 0.96, 1.0]);
            style
        }),
    );
    let _ = world.add_child(root, option);
    let _ = world.add_child(root, raycastable);
    let _ = world.add_child(root, payload);
    let _ = world.add_child(root, style);

    if let Some(icon) = icon_name {
        if let Some(expr) = build_editor_panel_component_expr(
            world,
            emit,
            icons_asset_path(),
            icon,
            vec![],
            PanelKind::Inspector,
            &format!("grid_{}_{}_icon", row_name, suffix),
        ) {
            let icon_wrapper = world.add_component_boxed_named(
                format!("{row_name}_{suffix}_icon_wrapper"),
                Box::new(TransformComponent::new().with_scale(0.25, 0.25, 0.333)),
            );
            let _ = world.add_child(root, icon_wrapper);
            let _ = spawn_tree(&expr, Some(icon_wrapper), world, emit);
        }
    } else {
        let text_root = world.add_component_boxed_named(
            format!("{row_name}_{suffix}_text_root"),
            Box::new(TransformComponent::new().with_position(0.0, 0.0, 0.005)),
        );
        let text_component = world.add_component_boxed_named(
            format!("{row_name}_{suffix}_text"),
            Box::new(TextComponent::new(text.to_string()).with_font_size(0.08)),
        );
        let _ = world.add_child(root, text_root);
        let _ = world.add_child(text_root, text_component);
    }

    root
}

fn clear_active_grid_selection_if_matches(
    world: &mut World,
    emit: &mut dyn SignalEmitter,
    panel_query_root: ComponentId,
    editor_context_state: &Arc<Mutex<EditorContextState>>,
    owner_transform: ComponentId,
) {
    let should_clear = editor_context_state
        .lock()
        .expect("editor context state mutex poisoned")
        .active_grid_owner_transform
        == Some(owner_transform);
    if !should_clear {
        return;
    }

    if let Some(selection_root) =
        world.find_component(panel_query_root, GRID_PANEL_SELECTION_SELECTOR)
    {
        crate::engine::ecs::system::selection_system::apply_selection_set(
            world,
            emit,
            selection_root,
            Vec::new(),
            None,
        );
    }

    if let Ok(mut editor_context) = editor_context_state.lock() {
        if editor_context.active_grid_owner_transform == Some(owner_transform) {
            editor_context.active_grid_owner_transform = None;
        }
    }
}

pub(crate) const EDITOR_WORKSPACE_GRIDS_CHANGED: &str = "EditorWorkspaceGridsChanged";

pub(crate) fn resolve_grid_panel_editor_root(
    world: &World,
    editor_context: &EditorContextState,
) -> Option<ComponentId> {
    if editor_context.active_editor.is_some() {
        return editor_context.active_editor;
    }

    let grids = GridSystem::new();
    for component_id in world.all_components() {
        if world
            .get_component_by_id_as::<EditorComponent>(component_id)
            .is_some()
            && !grids
                .enumerate_grids_for_editor(world, component_id)
                .is_empty()
        {
            return Some(component_id);
        }
    }

    world.all_components().find(|&component_id| {
        world
            .get_component_by_id_as::<EditorComponent>(component_id)
            .is_some()
    })
}

pub(crate) fn rerender_grid_panel_from_context(
    world: &mut World,
    emit: &mut dyn SignalEmitter,
    panel_query_root: ComponentId,
    editor_context: &EditorContextState,
    data_renderer: &mut DataRendererSystem,
) {
    let Some(grid_panel_root) = world.find_component(panel_query_root, GRID_PANEL_ROOT_SELECTOR)
    else {
        return;
    };
    let Some(content_slot) = world.find_component(grid_panel_root, PANEL_CONTENT_SLOT_SELECTOR)
    else {
        return;
    };
    let Some(editor_root) = resolve_grid_panel_editor_root(world, editor_context) else {
        data_renderer.clear_slot(world, emit, content_slot);
        return;
    };

    let grids = GridSystem::new();
    let model = build_grid_panel_model(
        world,
        &grids,
        editor_root,
        editor_context.active_grid_owner_transform,
    );
    let items = grid_panel_items(&model);
    let container =
        match data_renderer.render_list(world, emit, content_slot, &GRID_PANEL_ROW_SPEC, &items) {
            Ok(container) => container,
            Err(error) => {
                eprintln!("[InspectorSystem] grid panel content render error: {error}");
                return;
            }
        };

    let Some(selection_root) = world.find_component(grid_panel_root, GRID_PANEL_SELECTION_SELECTOR)
    else {
        return;
    };

    if let Some((index, _)) = model.rows.iter().enumerate().find(|(_, row)| row.selected) {
        if let Some(row_root) =
            world.find_component(container, &format!("#{GRID_PANEL_ITEM_PREFIX}{index}"))
        {
            apply_selection_set(
                world,
                emit,
                selection_root,
                vec![SelectionEntry {
                    index: Some(index),
                    component: row_root,
                }],
                Some(row_root),
            );
        } else {
            apply_selection_set(world, emit, selection_root, Vec::new(), None);
        }
    } else {
        apply_selection_set(world, emit, selection_root, Vec::new(), None);
    }
}

pub(crate) enum GridPanelClickOutcome {
    NotHandled,
    Handled,
}

pub(crate) fn handle_grid_panel_click(
    world: &mut World,
    emit: &mut dyn SignalEmitter,
    panel_query_root: ComponentId,
    renderable: ComponentId,
    editor_context_state: &Arc<Mutex<EditorContextState>>,
    installed_editor_roots: &Arc<Mutex<Vec<ComponentId>>>,
    data_renderer: &mut DataRendererSystem,
) -> GridPanelClickOutcome {
    let Some(grid_panel_root) = world.find_component(panel_query_root, GRID_PANEL_ROOT_SELECTOR)
    else {
        return GridPanelClickOutcome::NotHandled;
    };
    if !is_descendant_or_self(world, grid_panel_root, renderable) {
        return GridPanelClickOutcome::NotHandled;
    }

    let editor_context = editor_context_state
        .lock()
        .expect("editor context state mutex poisoned")
        .clone();
    let editor_root = editor_context.active_editor.or_else(|| {
        installed_editor_roots
            .lock()
            .expect("installed editor roots mutex poisoned")
            .first()
            .copied()
    });
    let Some(editor_root) = editor_root else {
        return GridPanelClickOutcome::Handled;
    };

    if let Some(add_button) = world.find_component(grid_panel_root, GRID_PANEL_ADD_BUTTON_SELECTOR)
        && is_descendant_or_self(world, add_button, renderable)
    {
        let _owner_transform = GridSystem::new().spawn_grid_for_editor(
            world,
            emit,
            editor_root,
            GridSpawnSpec::from_cursor_pose(
                editor_context.cursor_translation,
                editor_context.cursor_rotation,
                false,
            ),
        );
        emit.push_event(
            panel_query_root,
            EventSignal::DataEvent {
                name: EDITOR_WORKSPACE_GRIDS_CHANGED.to_string(),
                payload: Some(editor_root),
            },
        );
        rerender_grid_panel_from_context(
            world,
            emit,
            panel_query_root,
            &EditorContextState {
                active_editor: Some(editor_root),
                ..editor_context.clone()
            },
            data_renderer,
        );
        return GridPanelClickOutcome::Handled;
    }

    if let Some(action) = decode_panel_action_payload(
        world,
        renderable,
        GRID_PANEL_VISIBILITY_PAYLOAD_NAME,
        PanelKind::Grid,
        PanelActionKind::Toggle,
        None,
        None,
    ) && let Some(owner_transform) = action.target_component
    {
        let _ = GridSystem::new().toggle_grid_hidden(world, emit, owner_transform);
        emit.push_event(
            panel_query_root,
            EventSignal::DataEvent {
                name: EDITOR_WORKSPACE_GRIDS_CHANGED.to_string(),
                payload: Some(owner_transform),
            },
        );
        rerender_grid_panel_from_context(
            world,
            emit,
            panel_query_root,
            &editor_context,
            data_renderer,
        );
        return GridPanelClickOutcome::Handled;
    }

    if let Some(action) = decode_panel_action_payload(
        world,
        renderable,
        GRID_PANEL_VISUAL_SPACE_PAYLOAD_NAME,
        PanelKind::Grid,
        PanelActionKind::Toggle,
        None,
        None,
    ) && let Some(owner_transform) = action.target_component
    {
        let _ = GridSystem::new().toggle_grid_visual_space(world, emit, owner_transform);
        rerender_grid_panel_from_context(
            world,
            emit,
            panel_query_root,
            &editor_context,
            data_renderer,
        );
        return GridPanelClickOutcome::Handled;
    }

    if let Some(action) = decode_panel_action_payload(
        world,
        renderable,
        GRID_PANEL_ENABLED_PAYLOAD_NAME,
        PanelKind::Grid,
        PanelActionKind::Toggle,
        None,
        None,
    ) && let Some(owner_transform) = action.target_component
    {
        let _ = GridSystem::new().toggle_grid_enabled(world, emit, owner_transform);
        emit.push_event(
            panel_query_root,
            EventSignal::DataEvent {
                name: EDITOR_WORKSPACE_GRIDS_CHANGED.to_string(),
                payload: Some(owner_transform),
            },
        );
        rerender_grid_panel_from_context(
            world,
            emit,
            panel_query_root,
            &editor_context,
            data_renderer,
        );
        return GridPanelClickOutcome::Handled;
    }

    if let Some(action) = decode_panel_action_payload(
        world,
        renderable,
        GRID_PANEL_BINDING_PAYLOAD_NAME,
        PanelKind::Grid,
        PanelActionKind::Toggle,
        None,
        None,
    ) && let Some(owner_transform) = action.target_component
    {
        let target = selected_manipulation_target(world, editor_root);
        if let Some(target) = target
            && target != editor_root
            && world
                .get_component_by_id_as::<TransformComponent>(target)
                .is_some()
            && world
                .get_component_by_id_as::<EditorComponent>(target)
                .is_none()
            && GridSystem::new()
                .grid_owned_by_transform(world, target)
                .is_none()
        {
            if GridSystem::bound_grid_transform(world, target) == Some(owner_transform) {
                GridSystem::unbind_transform(world, target);
            } else {
                GridSystem::bind_transform_to_grid(world, target, owner_transform);
            }
        }
        rerender_grid_panel_from_context(
            world,
            emit,
            panel_query_root,
            &editor_context,
            data_renderer,
        );
        return GridPanelClickOutcome::Handled;
    }

    if let Some(action) = decode_panel_action_payload(
        world,
        renderable,
        GRID_PANEL_DELETE_PAYLOAD_NAME,
        PanelKind::Grid,
        PanelActionKind::Delete,
        None,
        None,
    ) && let Some(owner_transform) = action.target_component
    {
        if editor_context.selected_component == Some(owner_transform) {
            apply_editor_root_selection(world, editor_context_state, editor_root);
        }
        clear_active_grid_selection_if_matches(
            world,
            emit,
            panel_query_root,
            editor_context_state,
            owner_transform,
        );
        let _ = GridSystem::new().delete_grid(world, emit, owner_transform);
        emit.push_event(
            panel_query_root,
            EventSignal::DataEvent {
                name: EDITOR_WORKSPACE_GRIDS_CHANGED.to_string(),
                payload: Some(editor_root),
            },
        );
        // Let the queued workspace event refresh the grid projection after this
        // click returns. A synchronous full-workspace refresh here used to rebuild
        // every panel, and doing an immediate grid refresh as well would project it
        // twice for the same deletion.
        return GridPanelClickOutcome::Handled;
    }

    GridPanelClickOutcome::NotHandled
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::ecs::component::{EditorComponent, GridComponent, TransformComponent};

    #[test]
    fn startup_grids_are_hidden_and_show_on_first_toggle() {
        use crate::engine::ecs::component::OpacityComponent;
        use crate::engine::ecs::{CommandQueue, RxWorld};
        use crate::scripting::runner::MeowMeowRunner;

        for authored_example in [false, true] {
            let mut world = World::default();
            let grids = GridSystem::new();
            let mut emit = CommandQueue::new();
            let (editor, owner) = if authored_example {
                let path = concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/examples/paint-grids-desktop.mms"
                );
                let source = std::fs::read_to_string(path).unwrap();
                let out = MeowMeowRunner::eval_with_world_at_path(
                    &source,
                    Some(path),
                    &mut world,
                    &mut RxWorld::default(),
                    &mut emit,
                );
                assert!(out.errors.is_empty(), "{:?}", out.errors);
                let grid = world
                    .all_components()
                    .find(|&id| world.get_component_by_id_as::<GridComponent>(id).is_some())
                    .unwrap();
                let owner = GridSystem::grid_owner_transform(&world, grid).unwrap();
                let entry = grids.grid_owned_by_transform(&world, owner).unwrap();
                (entry.editor_root, owner)
            } else {
                let editor = world.add_component(EditorComponent::new());
                let owner = grids.ensure_default_grid(&mut world, &mut emit, editor);
                (editor, owner)
            };
            let model = build_grid_panel_model(&world, &grids, editor, Some(owner));
            assert_eq!(model.rows.len(), 1);
            assert!(!model.rows[0].shown, "startup control must say Hidden");
            assert!(model.rows[0].enabled);
            if let Some(opacity) = world.find_component(owner, "#grid_live_opacity") {
                assert_eq!(
                    world
                        .get_component_by_id_as::<OpacityComponent>(opacity)
                        .unwrap()
                        .opacity,
                    0.0
                );
            }
            assert!(grids.toggle_grid_hidden(&mut world, &mut emit, owner));
            assert!(build_grid_panel_model(&world, &grids, editor, Some(owner)).rows[0].shown);
            let opacity = world
                .find_component(owner, "#grid_live_opacity")
                .expect("first show must establish live runtime");
            assert_eq!(
                world
                    .get_component_by_id_as::<OpacityComponent>(opacity)
                    .unwrap()
                    .opacity,
                1.0
            );
            let live_root = world.find_component(owner, "#grid_live_root").unwrap();
            assert!(grids.toggle_grid_hidden(&mut world, &mut emit, owner));
            assert!(!build_grid_panel_model(&world, &grids, editor, Some(owner)).rows[0].shown);
            assert_eq!(
                world
                    .get_component_by_id_as::<OpacityComponent>(opacity)
                    .unwrap()
                    .opacity,
                0.0
            );
            assert_eq!(
                world.find_component(owner, "#grid_live_root"),
                Some(live_root)
            );
        }
    }

    #[test]
    fn reduce_grid_panel_state_tracks_selection_and_delete() {
        let mut world = World::default();
        let a = world.add_component(TransformComponent::new());
        let b = world.add_component(TransformComponent::new());
        let state = reduce_grid_panel_state(
            &GridPanelState::default(),
            &GridPanelEvent::SelectionChanged {
                editor_root: Some(a),
                selected_component: Some(b),
            },
        );
        assert_eq!(state.active_editor, Some(a));
        assert_eq!(state.selected_grid_transform, Some(b));

        let cleared =
            reduce_grid_panel_state(&state, &GridPanelEvent::GridDeleted { owner_transform: b });
        assert_eq!(cleared.selected_grid_transform, None);
    }

    #[test]
    fn build_grid_panel_model_marks_selected_transform_grid() {
        let mut world = World::default();
        let grids = GridSystem::new();
        let editor = world.add_component(EditorComponent::new());
        let transform =
            world.add_component_boxed_named("grid_1", Box::new(TransformComponent::new()));
        let grid = world.add_component(GridComponent::new(0.5));
        let _ = world.add_child(editor, transform);
        let _ = world.add_child(transform, grid);
        let model = build_grid_panel_model(&world, &grids, editor, Some(transform));
        assert_eq!(model.title, "Grids");
        assert_eq!(model.rows.len(), 1);
        assert_eq!(model.rows[0].label, "grid_1");
        assert!(model.rows[0].shown);
        assert!(model.rows[0].enabled);
        assert!(model.rows[0].selected);
        assert!(!model.rows[0].bound);
    }

    #[test]
    fn build_grid_panel_model_marks_grid_bound_to_selected_manipulation_target() {
        let mut world = World::default();
        let grids = GridSystem::new();
        let editor = world.add_component(EditorComponent::new());
        let grid_transform = world.add_component(TransformComponent::new());
        let grid = world.add_component(GridComponent::new(0.5));
        let target = world.add_component(TransformComponent::new());
        world.add_child(editor, grid_transform).unwrap();
        world.add_child(grid_transform, grid).unwrap();
        world.add_child(editor, target).unwrap();
        world
            .get_component_by_id_as_mut::<EditorComponent>(editor)
            .unwrap()
            .selected = Some(target);
        GridSystem::bind_transform_to_grid(&mut world, target, grid_transform);

        let model = build_grid_panel_model(&world, &grids, editor, None);
        assert_eq!(model.rows.len(), 1);
        assert!(model.rows[0].bound);
        assert!(!model.rows[0].selected);
    }
}
