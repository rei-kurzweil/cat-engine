use crate::engine::ecs::ComponentId;
use crate::engine::ecs::component::Component;
use crate::engine::graphics::bounds::Aabb;

/// Runtime-owned correction that places an authored visual transform inside
/// its owning layout item's content box.
///
/// Both fields are expressed in the visual transform's parent-local space.
/// The source bounds exclude this correction, so repeated layout passes do
/// not feed the previous placement back into intrinsic measurement.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayoutVisualPlacementComponent {
    pub source_bounds_parent_local: Aabb,
    pub translation_parent_local: [f32; 3],
}

impl LayoutVisualPlacementComponent {
    pub fn new(source_bounds_parent_local: Aabb, translation_parent_local: [f32; 3]) -> Self {
        Self {
            source_bounds_parent_local,
            translation_parent_local,
        }
    }
}

impl Component for LayoutVisualPlacementComponent {
    fn name(&self) -> &'static str {
        "layout_visual_placement"
    }

    fn set_id(&mut self, _component: ComponentId) {}

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
