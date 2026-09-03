/// Inline formatting context layout.
///
/// Handles `display: InlineBlock` items in a horizontal cursor with line wrap.
/// Items advance left-to-right until `cursor_x + item.margin_box_width > avail_w`,
/// at which point a new line box starts. Each line's height is the tallest
/// `margin_box_height` on that line.
///
/// Inline (text) items are not yet wired — only inline-block atomic units flow
/// here today. Mixing inline-block and block items in the same container falls
/// through to block layout (handled by the dispatcher in `mod.rs`).
use crate::engine::ecs::ComponentId;
use crate::engine::ecs::component::style::VerticalAlign;
use crate::engine::ecs::component::{StyleComponent, TransformComponent};
use crate::engine::ecs::{IntentValue, SignalEmitter, World};

use super::block::apply_text_align;
use super::box_model_viz::sync_box_model_viz;
use super::measure::{
    MeasuredItem, apply_text_color_for_item, apply_text_font_size_for_item,
    apply_text_wrap_for_item, measure_container_items, measure_items,
};
/// Run an inline formatting context layout pass for `layout_id`.
///
/// Each TC child is treated as an atomic inline-block: its measured
/// margin box becomes the cursor advance. Wrap occurs when the cursor
/// would exceed `available_width`.
pub fn layout(
    world: &mut World,
    emit: &mut dyn SignalEmitter,
    layout_id: ComponentId,
) -> (f32, f32) {
    let (items, avail_w_gu, _avail_h_gu, unit_scale) = measure_items(world, layout_id);
    let viz = super::block::layout_root_has_inspect(world, layout_id);
    let axis_scales = super::measure::layout_root_axis_scales(world, layout_id);
    let (_total_x_gu, total_y_gu) = layout_items(
        world,
        emit,
        &items,
        avail_w_gu,
        unit_scale,
        axis_scales,
        0,
        0,
        viz,
    );
    (avail_w_gu, total_y_gu)
}

/// Inline-formatting-context layout over a pre-measured item list.
///
/// `avail_w_gu` is the inline-axis budget (in glyph units) the parent
/// passes down — for the LayoutRoot case that's `LayoutComponent.available_width`;
/// for a nested block item that switches to inline flow, it's
/// `item.content_width_gu` of the enclosing block.
/// Returns `(total_x_gu, total_y_gu)` — the final cursor position in glyph units
/// after placing all items (useful for computing the total extents).
pub(crate) fn layout_items(
    world: &mut World,
    emit: &mut dyn SignalEmitter,
    items: &[MeasuredItem],
    avail_w_gu: f32,
    unit_scale: f32,
    axis_scales: (f32, f32),
    depth: i32,
    parent_depth: i32,
    viz: bool,
) -> (f32, f32) {
    let mut cursor_x_gu: f32 = 0.0;
    let mut cursor_y_gu: f32 = 0.0;
    let mut line_height_gu: f32 = 0.0;
    // Items are positioned while traversing the line, before its final height
    // is known. Retain their unaligned positions so `vertical_align` can
    // adjust shorter inline-blocks after the line's tallest item is measured.
    let mut line_items: Vec<(ComponentId, [f32; 3], [f32; 3], f32)> = Vec::new();
    let resolved_z = (depth - parent_depth) as f32 * super::LAYER_DISTANCE;

    for original in items {
        // Auto-width inline-block items consume the remaining inline-axis budget
        // on this line — re-measure with that as their available width so the
        // wrap test below sees the actual width, and intrinsic height
        // (text wrap, child layout) is computed at the final width.
        let item: MeasuredItem = if original.is_auto_width {
            let remaining = (avail_w_gu - cursor_x_gu).max(0.0);
            super::measure::measure_item(
                world,
                original.tc_id,
                remaining,
                Some(original.content_height_gu),
                unit_scale,
            )
        } else {
            original.clone()
        };
        let item = &item;

        // Wrap to a new line if this item won't fit and we're not at the line start.
        if cursor_x_gu > 0.0 && cursor_x_gu + item.margin_box_width_gu > avail_w_gu {
            apply_inline_vertical_align(world, emit, &line_items, line_height_gu, unit_scale);
            line_items.clear();
            cursor_y_gu += line_height_gu;
            cursor_x_gu = 0.0;
            line_height_gu = 0.0;
        }

        let content_origin_x_gu = cursor_x_gu + item.margin_left_gu + item.padding_left_gu;
        let content_origin_y_gu = cursor_y_gu + item.margin_top_gu + item.padding_top_gu;

        let tc_scale = world
            .get_component_by_id_as::<TransformComponent>(item.tc_id)
            .map(|tc| tc.transform.scale)
            .unwrap_or([1.0, 1.0, 1.0]);

        let composed_z = resolved_z;
        let translation = [
            content_origin_x_gu * unit_scale,
            -(content_origin_y_gu * unit_scale),
            composed_z,
        ];

        emit.push_intent_now(
            item.tc_id,
            IntentValue::UpdateTransform {
                component_id: item.tc_id,
                translation,
                rotation_quat_xyzw: [0.0, 0.0, 0.0, 1.0],
                scale: tc_scale,
            },
        );
        line_items.push((item.tc_id, translation, tc_scale, item.margin_box_height_gu));

        super::block::sync_layout_bounds(world, emit, item, unit_scale);

        apply_text_font_size_for_item(world, emit, item.tc_id, unit_scale);
        apply_text_wrap_for_item(world, emit, item.tc_id, item.content_width_gu, unit_scale);
        apply_text_color_for_item(world, emit, item.tc_id);

        // Background quad — share the block-flow implementation so
        // `Style { background_color }` works consistently for both
        // formatting contexts.
        super::block::sync_bg_quad(
            world,
            emit,
            item.tc_id,
            item.padding_left_gu,
            item.padding_top_gu,
            item.box_width_gu,
            item.box_height_gu,
            unit_scale,
        );
        super::block::sync_auto_text_lift(world, emit, item.tc_id);
        sync_box_model_viz(world, emit, item, unit_scale, viz);
        apply_text_align(
            world,
            emit,
            item.tc_id,
            item.content_width_gu,
            item.content_height_gu,
            unit_scale,
        );
        let content_root =
            super::block::sync_overflow_topology(world, emit, item.tc_id, item.content_height_gu);

        // Recurse into the item's own children using whichever formatting
        // context their `display` modes call for. Inline-block items can
        // host either inline children (more text/icons) or block children
        // (a stacked sub-tree); both must be honored.
        let nested_items =
            measure_container_items(world, content_root, item.content_width_gu, None, unit_scale);
        if !nested_items.is_empty() {
            let child_depth = if super::block::item_owns_layer(world, item.tc_id) {
                depth + 1
            } else {
                depth
            };
            super::layout_container_items(
                world,
                emit,
                item.tc_id,
                &nested_items,
                item.content_width_gu,
                Some(item.content_height_gu),
                unit_scale,
                axis_scales,
                child_depth,
                depth,
                viz,
            );
        }

        cursor_x_gu += item.margin_box_width_gu;
        if item.margin_box_height_gu > line_height_gu {
            line_height_gu = item.margin_box_height_gu;
        }
    }

    apply_inline_vertical_align(world, emit, &line_items, line_height_gu, unit_scale);

    (cursor_x_gu, cursor_y_gu + line_height_gu)
}

fn apply_inline_vertical_align(
    world: &World,
    emit: &mut dyn SignalEmitter,
    items: &[(ComponentId, [f32; 3], [f32; 3], f32)],
    line_height_gu: f32,
    unit_scale: f32,
) {
    for &(id, base_translation, scale, margin_box_height_gu) in items {
        let align = world
            .children_of(id)
            .iter()
            .find_map(|&child| {
                world
                    .get_component_by_id_as::<StyleComponent>(child)
                    .map(|style| style.vertical_align)
            })
            .unwrap_or(VerticalAlign::Auto);
        let offset_gu = match align {
            VerticalAlign::Auto | VerticalAlign::Top => 0.0,
            VerticalAlign::Middle => (line_height_gu - margin_box_height_gu).max(0.0) * 0.5,
            VerticalAlign::Bottom => (line_height_gu - margin_box_height_gu).max(0.0),
        };
        if offset_gu == 0.0 {
            continue;
        }
        let mut translation = base_translation;
        // Layout +Y is down the page while transform +Y is up, so moving an
        // inline item toward the line's bottom subtracts world-space Y.
        translation[1] -= offset_gu * unit_scale;
        emit.push_intent_now(
            id,
            IntentValue::UpdateTransform {
                component_id: id,
                translation,
                rotation_quat_xyzw: [0.0, 0.0, 0.0, 1.0],
                scale,
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::ecs::component::style::{Display, SizeDimension};
    use crate::engine::ecs::component::{LayoutComponent, StyleComponent};
    use crate::engine::ecs::rx::{EventSignal, IntentSignal};

    #[derive(Default)]
    struct TestEmitter {
        intents: Vec<IntentSignal>,
    }

    impl SignalEmitter for TestEmitter {
        fn push_event(&mut self, _: ComponentId, _: EventSignal) {}

        fn push_intent(&mut self, _: ComponentId, intent: IntentSignal) {
            self.intents.push(intent);
        }
    }

    fn inline_item(
        world: &mut World,
        name: &str,
        width: f32,
        height: f32,
        vertical_align: VerticalAlign,
    ) -> ComponentId {
        let item = world.add_component_boxed_named(name, Box::new(TransformComponent::new()));
        let style = world.add_component({
            let mut style = StyleComponent::new();
            style.display = Some(Display::InlineBlock);
            style.width = SizeDimension::GlyphUnits(width);
            style.height = SizeDimension::GlyphUnits(height);
            style.vertical_align = vertical_align;
            style
        });
        world.add_child(item, style).unwrap();
        item
    }

    fn last_translation(emitter: &TestEmitter, item: ComponentId) -> [f32; 3] {
        emitter
            .intents
            .iter()
            .rev()
            .find_map(|intent| match &intent.value {
                IntentValue::UpdateTransform {
                    component_id,
                    translation,
                    ..
                } if *component_id == item => Some(*translation),
                _ => None,
            })
            .expect("item transform update")
    }

    #[test]
    fn bottom_aligned_inline_block_moves_to_the_bottom_of_its_line() {
        let mut world = World::default();
        let root = world.add_component(LayoutComponent::new(20.0));
        let short = inline_item(&mut world, "short", 3.0, 2.0, VerticalAlign::Bottom);
        let tall = inline_item(&mut world, "tall", 3.0, 5.0, VerticalAlign::Top);
        world.add_child(root, short).unwrap();
        world.add_child(root, tall).unwrap();

        let mut emit = TestEmitter::default();
        layout(&mut world, &mut emit, root);

        assert_eq!(last_translation(&emit, tall), [3.0, 0.0, 0.0]);
        assert_eq!(last_translation(&emit, short), [0.0, -3.0, 0.0]);
    }
}
