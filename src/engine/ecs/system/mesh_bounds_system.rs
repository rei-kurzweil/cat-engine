use std::collections::HashMap;

use crate::engine::ecs::ComponentId;
use crate::engine::graphics::bounds::Aabb;
use crate::engine::graphics::primitives::TransformMatrix;

/// The type of runtime mesh output providing an aggregate bound.
///
/// This intentionally describes render outputs rather than authored ECS
/// components: some outputs (such as `CombineMesh`) are registered directly
/// in `VisualWorld` and do not have a `RenderableComponent` of their own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeshOutputKind {
    CombineMesh,
    ImplicitSurface,
}

/// Local bounds and placement for one mesh-backed runtime output.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MeshOutputBounds {
    pub owner: ComponentId,
    pub local: Aabb,
    pub model: TransformMatrix,
    pub kind: MeshOutputKind,
}

impl MeshOutputBounds {
    pub fn world(self) -> Aabb {
        self.local.transformed(self.model)
    }
}

/// Runtime registry for mesh-output bounds.
///
/// The editor bounds overlay and the future non-ECS spatial-instance path
/// consume this registry.  It deliberately has no authored-world side effects.
#[derive(Debug, Default)]
pub struct MeshBoundsSystem {
    outputs: HashMap<ComponentId, MeshOutputBounds>,
}

impl MeshBoundsSystem {
    pub fn register_or_update(
        &mut self,
        owner: ComponentId,
        local: Aabb,
        model: TransformMatrix,
        kind: MeshOutputKind,
    ) {
        self.outputs.insert(
            owner,
            MeshOutputBounds {
                owner,
                local,
                model,
                kind,
            },
        );
    }

    pub fn update_model(&mut self, owner: ComponentId, model: TransformMatrix) {
        if let Some(bounds) = self.outputs.get_mut(&owner) {
            bounds.model = model;
        }
    }

    pub fn remove(&mut self, owner: ComponentId) {
        self.outputs.remove(&owner);
    }

    pub fn output(&self, owner: ComponentId) -> Option<MeshOutputBounds> {
        self.outputs.get(&owner).copied()
    }

    pub fn outputs(&self) -> impl Iterator<Item = MeshOutputBounds> + '_ {
        self.outputs.values().copied()
    }
}

#[cfg(test)]
mod tests {
    use super::{MeshBoundsSystem, MeshOutputKind};
    use crate::engine::ecs::ComponentId;
    use crate::engine::graphics::bounds::Aabb;

    #[test]
    fn tracks_output_and_its_current_world_bounds() {
        let mut system = MeshBoundsSystem::default();
        let owner = ComponentId::default();
        let local = Aabb {
            min: [-1.0, -2.0, -3.0],
            max: [1.0, 2.0, 3.0],
        };
        let model = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [4.0, 5.0, 6.0, 1.0],
        ];
        system.register_or_update(owner, local, model, MeshOutputKind::CombineMesh);

        assert_eq!(system.output(owner).unwrap().world().min, [3.0, 3.0, 3.0]);
        assert_eq!(system.output(owner).unwrap().world().max, [5.0, 7.0, 9.0]);

        system.remove(owner);
        assert!(system.output(owner).is_none());
    }
}
