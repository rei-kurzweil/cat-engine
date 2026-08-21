//! Pure, editor-local grid gesture state and rasterisation rules.
//!
//! This module deliberately knows nothing about MMS or scene mutation.  It is
//! shared by paint tools so desktop and XR can produce the same address list
//! once their rays have been resolved.

use std::collections::HashSet;

use crate::engine::ecs::ComponentId;
use crate::engine::ecs::system::grid_system::{CapturedGrid, GridAddress, GridSystem};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GridGestureTool {
    FreeDraw,
    Line,
    Spray,
}

#[derive(Debug, Clone)]
pub struct GridGestureSession {
    pub pointer: ComponentId,
    pub grid: CapturedGrid,
    pub tool: GridGestureTool,
    pub brush_size: u8,
    pub start: GridAddress,
    pub current: GridAddress,
    emitted: HashSet<GridAddress>,
}

impl GridGestureSession {
    pub fn new(
        pointer: ComponentId,
        grid: CapturedGrid,
        tool: GridGestureTool,
        brush_size: u8,
        start: GridAddress,
    ) -> Self {
        Self {
            pointer,
            grid,
            tool,
            brush_size: brush_size.min(8),
            start,
            current: start,
            emitted: HashSet::new(),
        }
    }

    pub fn update(&mut self, current: GridAddress) -> Vec<GridAddress> {
        let previous = self.current;
        self.current = current;
        let candidates = match self.tool {
            GridGestureTool::FreeDraw => supercover_path(previous, current),
            GridGestureTool::Line => thin_line_path(self.start, current),
            GridGestureTool::Spray => disk_footprint(current, self.brush_size),
        };
        candidates
            .into_iter()
            .filter(|address| GridSystem::address_in_domain(&self.grid, *address))
            .filter(|address| self.emitted.insert(*address))
            .collect()
    }

    /// The complete, non-deduplicated current preview target.  Line uses this
    /// to reconcile previews when its endpoint changes or reverses.
    pub fn current_targets(&self) -> Vec<GridAddress> {
        let candidates = match self.tool {
            GridGestureTool::FreeDraw => supercover_path(self.start, self.current),
            GridGestureTool::Line => thin_line_path(self.start, self.current),
            GridGestureTool::Spray => disk_footprint(self.current, self.brush_size),
        };
        candidates
            .into_iter()
            .filter(|address| GridSystem::address_in_domain(&self.grid, *address))
            .collect()
    }
}

/// Ordered integer supercover: every cell touched by the centre-to-centre
/// segment is included. Corner crossings include both orthogonal neighbours.
pub fn supercover_path(start: GridAddress, end: GridAddress) -> Vec<GridAddress> {
    let dx = end.u - start.u;
    let dy = end.v - start.v;
    let nx = dx.abs();
    let ny = dy.abs();
    let sx = dx.signum();
    let sy = dy.signum();
    let mut out = vec![start];
    let (mut x, mut y) = (start.u, start.v);
    let (mut ix, mut iy) = (0, 0);
    while ix < nx || iy < ny {
        let lhs = (1 + 2 * ix) * ny;
        let rhs = (1 + 2 * iy) * nx;
        if lhs == rhs {
            if ix < nx {
                out.push(GridAddress { u: x + sx, v: y });
            }
            if iy < ny {
                out.push(GridAddress { u: x, v: y + sy });
            }
            x += sx;
            y += sy;
            ix += 1;
            iy += 1;
            out.push(GridAddress { u: x, v: y });
        } else if lhs < rhs {
            x += sx;
            ix += 1;
            out.push(GridAddress { u: x, v: y });
        } else {
            y += sy;
            iy += 1;
            out.push(GridAddress { u: x, v: y });
        }
    }
    deduplicate_ordered(out)
}

/// A thin 8-connected path. Exact error ties take a diagonal step.  The
/// canonical endpoint ordering makes `path(b, a) == reverse(path(a, b))`.
pub fn thin_line_path(a: GridAddress, b: GridAddress) -> Vec<GridAddress> {
    if a == b {
        return vec![a];
    }
    let reversed = (a.u, a.v) > (b.u, b.v);
    let (start, end) = if reversed { (b, a) } else { (a, b) };
    let dx = (end.u - start.u).abs();
    let dy = (end.v - start.v).abs();
    let sx = (end.u - start.u).signum();
    let sy = (end.v - start.v).signum();
    let mut err = dx - dy;
    let (mut x, mut y) = (start.u, start.v);
    let mut out = Vec::new();
    loop {
        out.push(GridAddress { u: x, v: y });
        if x == end.u && y == end.v {
            break;
        }
        let twice = 2 * err;
        // Equality takes both axes: the specified diagonal tie break.
        if twice >= -dy {
            err -= dy;
            x += sx;
        }
        if twice <= dx {
            err += dx;
            y += sy;
        }
    }
    if reversed {
        out.reverse();
    }
    out
}

/// Filled grid-local Euclidean disk. Radius zero contains only its centre.
pub fn disk_footprint(center: GridAddress, radius: u8) -> Vec<GridAddress> {
    let r = radius as i32;
    let mut out = Vec::new();
    for v in (center.v - r)..=(center.v + r) {
        for u in (center.u - r)..=(center.u + r) {
            let du = u - center.u;
            let dv = v - center.v;
            if du * du + dv * dv <= r * r {
                out.push(GridAddress { u, v });
            }
        }
    }
    out
}

fn deduplicate_ordered(values: Vec<GridAddress>) -> Vec<GridAddress> {
    let mut seen = HashSet::new();
    values
        .into_iter()
        .filter(|value| seen.insert(*value))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn a(u: i32, v: i32) -> GridAddress {
        GridAddress { u, v }
    }

    #[test]
    fn supercover_includes_corner_neighbours() {
        assert_eq!(
            supercover_path(a(0, 0), a(1, 1)),
            vec![a(0, 0), a(1, 0), a(0, 1), a(1, 1)]
        );
    }

    #[test]
    fn thin_line_is_reverse_symmetric_and_8_connected() {
        let forward = thin_line_path(a(-3, 2), a(5, -1));
        let mut reverse = thin_line_path(a(5, -1), a(-3, 2));
        reverse.reverse();
        assert_eq!(forward, reverse);
        assert!(forward.windows(2).all(|pair| {
            (pair[1].u - pair[0].u).abs() <= 1 && (pair[1].v - pair[0].v).abs() <= 1
        }));
    }

    #[test]
    fn disk_zero_and_one_are_deterministic() {
        assert_eq!(disk_footprint(a(2, 3), 0), vec![a(2, 3)]);
        assert_eq!(
            disk_footprint(a(0, 0), 1),
            vec![a(0, -1), a(-1, 0), a(0, 0), a(1, 0), a(0, 1)]
        );
    }
}
