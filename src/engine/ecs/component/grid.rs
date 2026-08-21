use crate::engine::ecs::{component::Component, ComponentId};

/// Coordinate frame used only to draw a grid. Snapping is always grid-local.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GridVisualSpace {
    #[default]
    Local,
    World,
}

impl GridVisualSpace {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::World => "world",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "local" => Some(Self::Local),
            "world" => Some(Self::World),
            _ => None,
        }
    }
}

/// Returns the two world-axis families least aligned with a plane normal.
/// Axis indices are X=0, Y=1, Z=2; ties retain X/Y/Z order.
pub fn world_coordinate_families(normal: [f32; 3]) -> [usize; 2] {
    let mut axes = [0usize, 1, 2];
    axes.sort_by(|&a, &b| normal[a].abs().total_cmp(&normal[b].abs()));
    [axes[0], axes[1]]
}

#[derive(Debug, Clone, Copy)]
pub struct GridComponent {
    pub spacing: f32,
    pub size_x: u32,
    pub size_z: u32,
    pub enabled: bool,
    pub hidden: bool,
    pub selectable: bool,
    pub visual_space: GridVisualSpace,
    component: Option<ComponentId>,
}

impl GridComponent {
    pub const DEFAULT_SIZE_X: u32 = 16;
    pub const DEFAULT_SIZE_Z: u32 = 16;

    pub fn new(spacing: f32) -> Self {
        Self {
            spacing: spacing.max(1e-4),
            size_x: Self::DEFAULT_SIZE_X,
            size_z: Self::DEFAULT_SIZE_Z,
            enabled: true,
            hidden: false,
            selectable: true,
            visual_space: GridVisualSpace::Local,
            component: None,
        }
    }

    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn with_selectable(mut self, selectable: bool) -> Self {
        self.selectable = selectable;
        self
    }

    pub fn with_hidden(mut self, hidden: bool) -> Self {
        self.hidden = hidden;
        self
    }

    pub fn with_spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing.max(1e-4);
        self
    }

    pub fn with_size_x(mut self, size_x: u32) -> Self {
        self.size_x = size_x.max(1);
        self
    }

    pub fn with_size_z(mut self, size_z: u32) -> Self {
        self.size_z = size_z.max(1);
        self
    }

    pub fn with_visual_space(mut self, visual_space: GridVisualSpace) -> Self {
        self.visual_space = visual_space;
        self
    }
}

impl Default for GridComponent {
    fn default() -> Self {
        Self::new(1.0)
    }
}

impl Component for GridComponent {
    fn set_id(&mut self, id: ComponentId) {
        self.component = Some(id);
    }

    fn name(&self) -> &'static str {
        "grid"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn to_mms_ast(
        &self,
        _world: &crate::engine::ecs::World,
    ) -> crate::scripting::ast::ComponentExpression {
        use crate::engine::ecs::component::ce_helpers::*;
        ce_call("Grid", "spacing", vec![num(self.spacing as f64)])
            .with_call("size_x", vec![num(self.size_x as f64)])
            .with_call("size_z", vec![num(self.size_z as f64)])
            .with_call("enabled", vec![b(self.enabled)])
            .with_call("hidden", vec![b(self.hidden)])
            .with_call("selectable", vec![b(self.selectable)])
            .with_call("visual_space", vec![s(self.visual_space.as_str())])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn world_coordinate_families_choose_non_degenerate_plane_axes() {
        assert_eq!(world_coordinate_families([0.0, 1.0, 0.0]), [0, 2]);
        assert_eq!(world_coordinate_families([0.0, 0.0, 1.0]), [0, 1]);
        assert_eq!(world_coordinate_families([1.0, 0.0, 0.0]), [1, 2]);
        assert_eq!(world_coordinate_families([0.5, 0.5, 0.9]), [0, 1]);
    }

    #[test]
    fn visual_space_defaults_to_local() {
        assert_eq!(
            GridComponent::default().visual_space,
            GridVisualSpace::Local
        );
        assert_eq!(
            GridVisualSpace::parse("world"),
            Some(GridVisualSpace::World)
        );
    }
}
