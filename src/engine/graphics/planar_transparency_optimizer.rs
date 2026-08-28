//! Cached, transaction-based classification for planar transparent content.
//!
//! This module deliberately has no renderer or ECS-system integration yet. Callers publish a
//! complete scope snapshot, and only a successful commit replaces the active classification.
//! `VisualWorld` will consume these resolutions in a later integration step.

use crate::engine::ecs::ComponentId;
use std::collections::HashMap;

/// Stable identity for one independent planar transparency scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PlanarTransparencyScopeId(pub ComponentId);

/// Opaque handle for one staged replacement of a scope snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PlanarTransparencyTransaction {
    scope: PlanarTransparencyScopeId,
    generation: u64,
}

/// Axis-aligned rectangle expressed in a scope's local planar coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlanarRect {
    pub min_x: f32,
    pub min_y: f32,
    pub max_x: f32,
    pub max_y: f32,
}

impl PlanarRect {
    /// Returns whether both rectangles overlap with positive area.
    ///
    /// Touching only at an edge or corner is not an overlap.
    fn overlaps_open_area(self, other: Self) -> bool {
        self.min_x < other.max_x
            && other.min_x < self.max_x
            && self.min_y < other.max_y
            && other.min_y < self.max_y
    }

    fn is_valid(self) -> bool {
        self.min_x.is_finite()
            && self.min_y.is_finite()
            && self.max_x.is_finite()
            && self.max_y.is_finite()
            && self.min_x <= self.max_x
            && self.min_y <= self.max_y
    }
}

/// One generated or system-owned transparent renderable in a planar scope.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlanarTransparencyCandidate {
    /// The nested `RenderableComponent` identity, not its owning transform.
    pub renderable: ComponentId,
    pub root_local_rect: PlanarRect,
    /// Relative depth along the scope normal. It is retained for the later renderer join.
    pub stacking_depth: f32,
    /// Deterministic layout/system order used to break equal-depth ties.
    pub stable_order: u32,
}

/// Renderer-facing result of a committed planar classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanarTransparencyResolution {
    /// No other candidate in this scope overlaps this candidate's rectangle.
    SingleLayer,
    /// This candidate is in a connected overlapping group, or was conservatively classified.
    MultiLayer,
}

/// A committed candidate result retained until its scope is replaced or removed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResolvedPlanarTransparencyCandidate {
    pub resolution: PlanarTransparencyResolution,
    pub stacking_depth: f32,
    pub stable_order: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanarTransparencyTransactionError {
    StaleTransaction,
    DuplicateCandidate { renderable: ComponentId },
    InvalidCandidate { renderable: ComponentId },
}

#[derive(Debug, Default)]
struct ActiveScope {
    candidates: HashMap<ComponentId, ResolvedPlanarTransparencyCandidate>,
}

#[derive(Debug)]
struct StagedScope {
    generation: u64,
    candidates: HashMap<ComponentId, PlanarTransparencyCandidate>,
}

/// Pure CPU optimizer for declared planar transparency scopes.
///
/// A new transaction supersedes any earlier uncommitted transaction for the same scope. Until
/// commit succeeds, active scope results remain unchanged.
#[derive(Debug, Default)]
pub struct PlanarTransparencyOptimizer {
    active_scopes: HashMap<PlanarTransparencyScopeId, ActiveScope>,
    staged_scopes: HashMap<PlanarTransparencyScopeId, StagedScope>,
    next_generation: HashMap<PlanarTransparencyScopeId, u64>,
}

impl PlanarTransparencyOptimizer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Begins a staged full replacement for `scope`.
    pub fn begin_scope_update(
        &mut self,
        scope: PlanarTransparencyScopeId,
    ) -> PlanarTransparencyTransaction {
        let next = self.next_generation.entry(scope).or_insert(0);
        *next = next.saturating_add(1);
        let generation = *next;
        self.staged_scopes.insert(
            scope,
            StagedScope {
                generation,
                candidates: HashMap::new(),
            },
        );
        PlanarTransparencyTransaction { scope, generation }
    }

    /// Adds one candidate to the transaction's staged replacement snapshot.
    pub fn add_candidate(
        &mut self,
        transaction: PlanarTransparencyTransaction,
        candidate: PlanarTransparencyCandidate,
    ) -> Result<(), PlanarTransparencyTransactionError> {
        let staged = self.staged_for_transaction_mut(transaction)?;
        if staged.candidates.contains_key(&candidate.renderable) {
            return Err(PlanarTransparencyTransactionError::DuplicateCandidate {
                renderable: candidate.renderable,
            });
        }
        staged.candidates.insert(candidate.renderable, candidate);
        Ok(())
    }

    /// Atomically replaces the active scope snapshot and returns its resolved candidate count.
    pub fn commit_scope_update(
        &mut self,
        transaction: PlanarTransparencyTransaction,
    ) -> Result<usize, PlanarTransparencyTransactionError> {
        let staged = self.staged_for_transaction(transaction)?;
        for candidate in staged.candidates.values() {
            if !candidate.root_local_rect.is_valid() || !candidate.stacking_depth.is_finite() {
                return Err(PlanarTransparencyTransactionError::InvalidCandidate {
                    renderable: candidate.renderable,
                });
            }
        }

        let staged = self
            .staged_scopes
            .remove(&transaction.scope)
            .expect("validated staged transaction must still be present");
        let active = ActiveScope {
            candidates: resolve_scope_candidates(&staged.candidates),
        };
        let candidate_count = active.candidates.len();
        self.active_scopes.insert(transaction.scope, active);
        Ok(candidate_count)
    }

    /// Removes both committed and staged state for a destroyed planar scope.
    pub fn remove_scope(&mut self, scope: PlanarTransparencyScopeId) {
        self.active_scopes.remove(&scope);
        self.staged_scopes.remove(&scope);
        self.next_generation.remove(&scope);
    }

    /// Returns a committed resolution. Staged data is intentionally invisible here.
    pub fn resolution_for(
        &self,
        scope: PlanarTransparencyScopeId,
        renderable: ComponentId,
    ) -> Option<ResolvedPlanarTransparencyCandidate> {
        self.active_scopes
            .get(&scope)
            .and_then(|active| active.candidates.get(&renderable))
            .copied()
    }

    pub fn committed_candidate_count(&self, scope: PlanarTransparencyScopeId) -> usize {
        self.active_scopes
            .get(&scope)
            .map_or(0, |active| active.candidates.len())
    }

    fn staged_for_transaction(
        &self,
        transaction: PlanarTransparencyTransaction,
    ) -> Result<&StagedScope, PlanarTransparencyTransactionError> {
        let Some(staged) = self.staged_scopes.get(&transaction.scope) else {
            return Err(PlanarTransparencyTransactionError::StaleTransaction);
        };
        if staged.generation != transaction.generation {
            return Err(PlanarTransparencyTransactionError::StaleTransaction);
        }
        Ok(staged)
    }

    fn staged_for_transaction_mut(
        &mut self,
        transaction: PlanarTransparencyTransaction,
    ) -> Result<&mut StagedScope, PlanarTransparencyTransactionError> {
        let Some(staged) = self.staged_scopes.get_mut(&transaction.scope) else {
            return Err(PlanarTransparencyTransactionError::StaleTransaction);
        };
        if staged.generation != transaction.generation {
            return Err(PlanarTransparencyTransactionError::StaleTransaction);
        }
        Ok(staged)
    }
}

fn resolve_scope_candidates(
    candidates: &HashMap<ComponentId, PlanarTransparencyCandidate>,
) -> HashMap<ComponentId, ResolvedPlanarTransparencyCandidate> {
    let mut overlapping = HashMap::<ComponentId, bool>::new();
    let candidates: Vec<_> = candidates.values().copied().collect();

    for (i, candidate) in candidates.iter().enumerate() {
        for other in &candidates[i + 1..] {
            if candidate
                .root_local_rect
                .overlaps_open_area(other.root_local_rect)
            {
                overlapping.insert(candidate.renderable, true);
                overlapping.insert(other.renderable, true);
            }
        }
    }

    candidates
        .into_iter()
        .map(|candidate| {
            let resolution = if overlapping.contains_key(&candidate.renderable) {
                PlanarTransparencyResolution::MultiLayer
            } else {
                PlanarTransparencyResolution::SingleLayer
            };
            (
                candidate.renderable,
                ResolvedPlanarTransparencyCandidate {
                    resolution,
                    stacking_depth: candidate.stacking_depth,
                    stable_order: candidate.stable_order,
                },
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use slotmap::SlotMap;

    fn ids(count: usize) -> Vec<ComponentId> {
        let mut ids = SlotMap::<ComponentId, ()>::with_key();
        (0..count).map(|_| ids.insert(())).collect()
    }

    fn scope(id: ComponentId) -> PlanarTransparencyScopeId {
        PlanarTransparencyScopeId(id)
    }

    fn candidate(
        renderable: ComponentId,
        rect: PlanarRect,
        stable_order: u32,
    ) -> PlanarTransparencyCandidate {
        PlanarTransparencyCandidate {
            renderable,
            root_local_rect: rect,
            stacking_depth: stable_order as f32,
            stable_order,
        }
    }

    fn rect(min_x: f32, min_y: f32, max_x: f32, max_y: f32) -> PlanarRect {
        PlanarRect {
            min_x,
            min_y,
            max_x,
            max_y,
        }
    }

    #[test]
    fn staged_candidates_are_invisible_until_commit() {
        let [scope_id, renderable] = ids(2).try_into().unwrap();
        let scope = scope(scope_id);
        let mut optimizer = PlanarTransparencyOptimizer::new();
        let transaction = optimizer.begin_scope_update(scope);
        optimizer
            .add_candidate(
                transaction,
                candidate(renderable, rect(0.0, 0.0, 1.0, 1.0), 0),
            )
            .unwrap();

        assert_eq!(optimizer.resolution_for(scope, renderable), None);
        optimizer.commit_scope_update(transaction).unwrap();
        assert_eq!(
            optimizer
                .resolution_for(scope, renderable)
                .unwrap()
                .resolution,
            PlanarTransparencyResolution::SingleLayer
        );
    }

    #[test]
    fn newer_transaction_makes_the_old_one_stale() {
        let [scope_id, first, second] = ids(3).try_into().unwrap();
        let scope = scope(scope_id);
        let mut optimizer = PlanarTransparencyOptimizer::new();
        let old = optimizer.begin_scope_update(scope);
        let new = optimizer.begin_scope_update(scope);

        assert_eq!(
            optimizer.add_candidate(old, candidate(first, rect(0.0, 0.0, 1.0, 1.0), 0)),
            Err(PlanarTransparencyTransactionError::StaleTransaction)
        );
        optimizer
            .add_candidate(new, candidate(second, rect(0.0, 0.0, 1.0, 1.0), 0))
            .unwrap();
        optimizer.commit_scope_update(new).unwrap();
        assert_eq!(optimizer.resolution_for(scope, first), None);
        assert!(optimizer.resolution_for(scope, second).is_some());
    }

    #[test]
    fn commit_replaces_scope_and_removes_omitted_candidates() {
        let [scope_id, first, second] = ids(3).try_into().unwrap();
        let scope = scope(scope_id);
        let mut optimizer = PlanarTransparencyOptimizer::new();
        let first_commit = optimizer.begin_scope_update(scope);
        optimizer
            .add_candidate(first_commit, candidate(first, rect(0.0, 0.0, 1.0, 1.0), 0))
            .unwrap();
        optimizer
            .add_candidate(first_commit, candidate(second, rect(2.0, 0.0, 3.0, 1.0), 1))
            .unwrap();
        optimizer.commit_scope_update(first_commit).unwrap();

        let replacement = optimizer.begin_scope_update(scope);
        optimizer
            .add_candidate(replacement, candidate(second, rect(2.0, 0.0, 3.0, 1.0), 1))
            .unwrap();
        optimizer.commit_scope_update(replacement).unwrap();

        assert_eq!(optimizer.committed_candidate_count(scope), 1);
        assert_eq!(optimizer.resolution_for(scope, first), None);
        assert!(optimizer.resolution_for(scope, second).is_some());
    }

    #[test]
    fn overlap_and_edge_contact_resolve_as_specified() {
        let [
            scope_id,
            isolated,
            overlapping_a,
            overlapping_b,
            edge_touching,
        ] = ids(5).try_into().unwrap();
        let scope = scope(scope_id);
        let mut optimizer = PlanarTransparencyOptimizer::new();
        let transaction = optimizer.begin_scope_update(scope);
        optimizer
            .add_candidate(
                transaction,
                candidate(isolated, rect(4.0, 0.0, 5.0, 1.0), 0),
            )
            .unwrap();
        optimizer
            .add_candidate(
                transaction,
                candidate(overlapping_a, rect(0.0, 0.0, 2.0, 2.0), 1),
            )
            .unwrap();
        optimizer
            .add_candidate(
                transaction,
                candidate(overlapping_b, rect(1.0, 1.0, 3.0, 3.0), 2),
            )
            .unwrap();
        optimizer
            .add_candidate(
                transaction,
                candidate(edge_touching, rect(3.0, 0.0, 4.0, 1.0), 3),
            )
            .unwrap();
        optimizer.commit_scope_update(transaction).unwrap();

        assert_eq!(
            optimizer
                .resolution_for(scope, isolated)
                .unwrap()
                .resolution,
            PlanarTransparencyResolution::SingleLayer
        );
        assert_eq!(
            optimizer
                .resolution_for(scope, overlapping_a)
                .unwrap()
                .resolution,
            PlanarTransparencyResolution::MultiLayer
        );
        assert_eq!(
            optimizer
                .resolution_for(scope, overlapping_b)
                .unwrap()
                .resolution,
            PlanarTransparencyResolution::MultiLayer
        );
        assert_eq!(
            optimizer
                .resolution_for(scope, edge_touching)
                .unwrap()
                .resolution,
            PlanarTransparencyResolution::SingleLayer
        );
    }

    #[test]
    fn invalid_commit_preserves_the_previous_active_snapshot() {
        let [scope_id, valid, invalid] = ids(3).try_into().unwrap();
        let scope = scope(scope_id);
        let mut optimizer = PlanarTransparencyOptimizer::new();
        let initial = optimizer.begin_scope_update(scope);
        optimizer
            .add_candidate(initial, candidate(valid, rect(0.0, 0.0, 1.0, 1.0), 0))
            .unwrap();
        optimizer.commit_scope_update(initial).unwrap();

        let invalid_transaction = optimizer.begin_scope_update(scope);
        optimizer
            .add_candidate(
                invalid_transaction,
                candidate(invalid, rect(2.0, 0.0, 1.0, 1.0), 1),
            )
            .unwrap();
        assert_eq!(
            optimizer.commit_scope_update(invalid_transaction),
            Err(PlanarTransparencyTransactionError::InvalidCandidate {
                renderable: invalid
            })
        );
        assert!(optimizer.resolution_for(scope, valid).is_some());
        assert_eq!(optimizer.resolution_for(scope, invalid), None);
    }
}
