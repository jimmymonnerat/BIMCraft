//! What a plan draws of a mesh seen from above, as a draughtsman draws furniture and fixtures: the edges where its surface
//! ends, folds by [`FOLD`] or more, or turns away from the eye, as far as no face of the mesh over them hides them. The facets
//! of a smooth surface (a basin's bowl, a cushion) draw no lines between them. A form a file brings as triangles is drawn so
//! in plan (the `mesh` of a [`crate::family::Piece`]); [`seen`] keeps what it worked out, so a plan drawn again doesn't redo it.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use crate::geom::P2;
use crate::view3d::{P3, cross, dot, sub};

/// Degrees two faces fold at, or more, for the edge between them to be drawn: a fold, not a facet of a curve.
const FOLD: f64 = 30.0;
/// How far, mm, an edge must lie under a face for the face to hide it: one in the face, or on it, is seen.
const UNDER: f64 = 0.5;
/// How far, mm, a point may lie outside a face seen from above and still be under it: an edge under another's is hidden.
const BESIDE: f64 = 0.01;
/// The shortest line drawn, mm.
const SHORTEST: f64 = 0.05;
/// Corners closer than a hundredth of a millimetre are one.
const WELD: f64 = 100.0;
/// How many meshes' lines are kept for plans drawn again.
const KEPT: usize = 4096;
/// Cells a side of the grid that finds the faces over an edge has, at most.
const CELLS: f64 = 128.0;
/// A face this near upright (the upward part of its normal) hides nothing: seen from above it is hardly more than a line.
const STEEP: f64 = 1e-3;
/// How far, mm, a seam is looked past for a face of the same plane, and how near that plane a point must be to be on it.
const SEAM: f64 = 0.05;
/// Degrees a softer fold (from half of [`FOLD`]) may turn from a line drawn and still be drawn as its going on.
const ON: f64 = 40.0;

/// The lines a mesh is drawn with, shared by every plan that draws it.
pub type Lines = Arc<Vec<[P2; 2]>>;

/// The lines of `tris` seen from above, as [`edges`] finds them: worked out once for each mesh, then kept.
pub fn seen(tris: &[[P3; 3]]) -> Lines {
    static MEMO: OnceLock<Mutex<HashMap<u64, Lines>>> = OnceLock::new();
    let memo = MEMO.get_or_init(Default::default);
    let key = fingerprint(tris);
    if let Some(found) = memo.lock().ok().and_then(|m| m.get(&key).cloned()) {
        return found;
    }
    let lines = Arc::new(edges(tris));
    if let Ok(mut m) = memo.lock() {
        if m.len() >= KEPT {
            m.clear();
        }
        m.insert(key, Arc::clone(&lines));
    }
    lines
}

fn fingerprint(tris: &[[P3; 3]]) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    tris.len().hash(&mut h);
    for v in tris.iter().flatten().flatten() {
        v.to_bits().hash(&mut h);
    }
    h.finish()
}

/// The lines a plan draws of `tris` seen from above, in plan: each edge that bounds the surface, folds by [`FOLD`] or more, or
/// is where the surface turns away from the eye (its two faces on one side of it, seen from above), but for the stretches of
/// it a face of the mesh over it hides. A corner that isn't a number, and a face of no area, are left out.
pub fn edges(tris: &[[P3; 3]]) -> Vec<[P2; 2]> {
    let flat = |p: P3| P2(p[0], p[1]);
    // Corners welded, so faces that share an edge are found by it.
    let mut at: HashMap<[i64; 3], usize> = HashMap::new();
    let mut corners: Vec<P3> = Vec::new();
    let mut faces: Vec<[usize; 3]> = Vec::new();
    let mut normals: Vec<P3> = Vec::new();
    for t in tris {
        if t.iter().flatten().any(|v| !v.is_finite() || v.abs() > 1e12) {
            continue;
        }
        let f = t.map(|p| {
            let k = p.map(|v| (v * WELD).round() as i64);
            *at.entry(k).or_insert_with(|| {
                corners.push(p);
                corners.len() - 1
            })
        });
        if f[0] == f[1] || f[1] == f[2] || f[0] == f[2] {
            continue;
        }
        let n = cross(sub(corners[f[1]], corners[f[0]]), sub(corners[f[2]], corners[f[0]]));
        let len = dot(n, n).sqrt();
        if len.is_nan() || len <= 1e-9 {
            continue;
        }
        faces.push(f);
        normals.push(n.map(|v| v / len));
    }
    let mut sides: HashMap<(usize, usize), Vec<usize>> = HashMap::new();
    for (i, f) in faces.iter().enumerate() {
        for k in 0..3 {
            let (a, b) = (f[k], f[(k + 1) % 3]);
            sides.entry((a.min(b), a.max(b))).or_default().push(i);
        }
    }
    // The flat regions of the surface: faces of one plane that touch, corner to corner.
    let mut region: Vec<usize> = (0..faces.len()).collect();
    fn root(region: &mut [usize], mut f: usize) -> usize {
        while region[f] != f {
            region[f] = region[region[f]];
            f = region[f];
        }
        f
    }
    let mut at_corner: HashMap<usize, Vec<usize>> = HashMap::new();
    for (i, f) in faces.iter().enumerate() {
        for k in f {
            at_corner.entry(*k).or_default().push(i);
        }
    }
    for touching in at_corner.values() {
        for (i, &f) in touching.iter().enumerate() {
            for &g in &touching[i + 1..] {
                if dot(normals[f], normals[g]).abs() > 1.0 - 1e-6 {
                    let (rf, rg) = (root(&mut region, f), root(&mut region, g));
                    region[rf] = rg;
                }
            }
        }
    }
    let region: Vec<usize> = (0..faces.len()).map(|f| root(&mut region, f)).collect();
    // The faces that can hide an edge, those seen as more than a line from above, by the cells of a grid they cover.
    let Some((lo, hi)) = corners.iter().map(|p| flat(*p)).fold(None, |b: Option<(P2, P2)>, p| match b {
        None => Some((p, p)),
        Some((lo, hi)) => Some((P2(lo.0.min(p.0), lo.1.min(p.1)), P2(hi.0.max(p.0), hi.1.max(p.1)))),
    }) else {
        return Vec::new();
    };
    let cell = ((hi.0 - lo.0).max(hi.1 - lo.1) / CELLS).max(1.0);
    let (cols, rows) = (((hi.0 - lo.0) / cell) as usize + 1, ((hi.1 - lo.1) / cell) as usize + 1);
    let span = |a: f64, b: f64, origin: f64, n: usize| {
        let (a, b) = (a.min(b), a.max(b));
        let to = |x: f64| (((x - origin) / cell).floor().max(0.0) as usize).min(n - 1);
        (to(a - BESIDE), to(b + BESIDE))
    };
    let mut grid: Vec<Vec<usize>> = vec![Vec::new(); cols * rows];
    for (i, f) in faces.iter().enumerate().filter(|(i, _)| normals[*i][2].abs() > STEEP) {
        let ps = f.map(|k| flat(corners[k]));
        let (x0, x1) = span(ps[0].0.min(ps[1].0).min(ps[2].0), ps[0].0.max(ps[1].0).max(ps[2].0), lo.0, cols);
        let (y0, y1) = span(ps[0].1.min(ps[1].1).min(ps[2].1), ps[0].1.max(ps[1].1).max(ps[2].1), lo.1, rows);
        for y in y0..=y1 {
            for x in x0..=x1 {
                grid[y * cols + x].push(i);
            }
        }
    }
    let third = |f: usize, a: usize, b: usize| faces[f].into_iter().find(|k| *k != a && *k != b).unwrap_or(a);
    // Where face `f` goes on from its edge from `a` to `b`, `t` along it: a hair past the edge, away from the face, in its plane.
    let past = |f: usize, a: usize, b: usize, t: f64| {
        let (pa, pb) = (corners[a], corners[b]);
        let e = sub(pb, pa);
        let d = sub(corners[third(f, a, b)], pa);
        let k = dot(d, e) / dot(e, e);
        let away = [d[0] - e[0] * k, d[1] - e[1] * k, d[2] - e[2] * k];
        let len = dot(away, away).sqrt();
        (len > 0.0).then(|| [0, 1, 2].map(|i| pa[i] + e[i] * t - away[i] / len * SEAM))
    };
    // A face's edge its flat region goes on past, along most of it: a seam of one surface, where its faces meet without
    // sharing their corners (one's corner on the other's side), or where a fin stands in it; not where the surface ends.
    let seam = |f: usize, a: usize, b: usize| {
        let n = normals[f];
        let covered = [0.1, 0.3, 0.5, 0.7, 0.9]
            .into_iter()
            .filter(|t| {
                let Some(q) = past(f, a, b, *t) else { return false };
                let (x, _) = span(q[0], q[0], lo.0, cols);
                let (y, _) = span(q[1], q[1], lo.1, rows);
                grid[y * cols + x].iter().any(|&g| {
                    g != f
                        && region[g] == region[f]
                        && dot(sub(q, corners[faces[g][0]]), normals[g]).abs() < SEAM
                        && dot(normals[g], n).abs() > 1.0 - 1e-6
                        && inside(faces[g].map(|k| flat(corners[k])), flat(q))
                })
            })
            .count();
        covered >= 3
    };
    // How far from flat, degrees, two faces go on from each other: their corners off the edge, square to it, point apart
    // when they're flat.
    let bend = |a: usize, b: usize, f: usize, g: usize| {
        let (pa, pb) = (corners[a], corners[b]);
        let e = sub(pb, pa);
        let ee = dot(e, e);
        let off = |r: usize| {
            let d = sub(corners[r], pa);
            let k = dot(d, e) / ee;
            [d[0] - e[0] * k, d[1] - e[1] * k, d[2] - e[2] * k]
        };
        let (u, v) = (off(third(f, a, b)), off(third(g, a, b)));
        let (uu, vv) = (dot(u, u).sqrt(), dot(v, v).sqrt());
        if uu * vv > 0.0 { 180.0 - (dot(u, v) / (uu * vv)).clamp(-1.0, 1.0).acos().to_degrees() } else { 0.0 }
    };
    let upright = |f: usize| normals[f][2].abs() < 1e-6;
    let mut drawn: Vec<(usize, usize)> = Vec::new();
    let mut soft: Vec<(usize, usize)> = Vec::new();
    for (&(a, b), on) in &sides {
        let lying: Vec<usize> = on.iter().copied().filter(|f| !upright(*f)).collect();
        match (on.as_slice(), lying.len()) {
            ([f, g], 2) => {
                let folds = bend(a, b, *f, *g);
                // Seen from above, both faces on one side of the edge: the surface turns away there.
                let (p, q) = (flat(corners[a]), flat(corners[b]));
                let d = q - p;
                let tiny = 1e-9 * (1.0 + d.length());
                let (s1, s2) = (d.cross(flat(corners[third(*f, a, b)]) - p), d.cross(flat(corners[third(*g, a, b)]) - p));
                let turns = (s1 > tiny && s2 > tiny) || (s1 < -tiny && s2 < -tiny);
                if folds >= FOLD || turns {
                    drawn.push((a, b));
                } else if folds >= FOLD / 2.0 {
                    soft.push((a, b));
                }
            }
            // Upright faces alone, seen edge on: a fold between two, or where one ends or three meet.
            (_, 0) => {
                if !matches!(on.as_slice(), [f, g] if bend(a, b, *f, *g) < FOLD) {
                    drawn.push((a, b));
                }
            }
            // Where a lying surface ends, or meets an upright face: drawn unless the surface goes on past it in its plane (a
            // seam where its faces meet without sharing their corners, or a fin standing in it, as a counter top with a sink's
            // hole may be written).
            _ => {
                if !lying.iter().all(|f| seam(*f, a, b)) {
                    drawn.push((a, b));
                }
            }
        }
    }
    // A fold that softens for a stretch (a cushion's seam) is drawn on through it: a softer fold that goes on from a line
    // drawn, in its direction, is drawn too.
    let mut ends: HashMap<usize, Vec<usize>> = HashMap::new();
    for &(a, b) in &drawn {
        ends.entry(a).or_default().push(b);
        ends.entry(b).or_default().push(a);
    }
    let goes_on = |from: usize, to: usize, ends: &HashMap<usize, Vec<usize>>| {
        let d = sub(corners[to], corners[from]);
        ends.get(&from).is_some_and(|others| {
            others.iter().any(|&o| {
                let back = sub(corners[from], corners[o]);
                dot(d, back) > (dot(d, d) * dot(back, back)).sqrt() * ON.to_radians().cos()
            })
        })
    };
    soft.sort_unstable();
    for _ in 0..64 {
        let before = drawn.len();
        soft.retain(|&(a, b)| {
            if goes_on(a, b, &ends) || goes_on(b, a, &ends) {
                drawn.push((a, b));
                ends.entry(a).or_default().push(b);
                ends.entry(b).or_default().push(a);
                false
            } else {
                true
            }
        });
        if drawn.len() == before {
            break;
        }
    }
    drawn.sort_unstable();
    let mut stamp = vec![usize::MAX; faces.len()];
    let mut out: Vec<[P2; 2]> = Vec::new();
    for (n, &(a, b)) in drawn.iter().enumerate() {
        let (pa, pb) = (corners[a], corners[b]);
        let (p, q) = (flat(pa), flat(pb));
        let d = q - p;
        let len = d.length();
        if len < SHORTEST {
            continue;
        }
        let mut hidden: Vec<(f64, f64)> = Vec::new();
        let (x0, x1) = span(p.0, q.0, lo.0, cols);
        let (y0, y1) = span(p.1, q.1, lo.1, rows);
        for y in y0..=y1 {
            for x in x0..=x1 {
                for &fi in &grid[y * cols + x] {
                    if stamp[fi] == n {
                        continue;
                    }
                    stamp[fi] = n;
                    // A face at either end of it goes on from it: it hides none of it.
                    let f = faces[fi];
                    if f.contains(&a) || f.contains(&b) {
                        continue;
                    }
                    if let Some(h) = under(f.map(|k| corners[k]), normals[fi], pa, pb) {
                        hidden.push(h);
                    }
                }
            }
        }
        hidden.sort_by(|x, y| x.0.total_cmp(&y.0));
        let mut from = 0.0;
        let mut piece = |t0: f64, t1: f64| {
            if (t1 - t0) * len >= SHORTEST {
                out.push([p + d * t0, p + d * t1]);
            }
        };
        for (s0, s1) in hidden {
            if s0 > from {
                piece(from, s0);
            }
            from = f64::max(from, s1);
        }
        if from < 1.0 {
            piece(from, 1.0);
        }
    }
    // The same line twice (two edges one over the other, both seen) is drawn once.
    let key = |s: &[P2; 2]| {
        let k = |p: P2| ((p.0 * WELD).round() as i64, (p.1 * WELD).round() as i64);
        let (a, b) = (k(s[0]), k(s[1]));
        if a <= b { (a, b) } else { (b, a) }
    };
    let mut once = std::collections::HashSet::new();
    out.retain(|s| once.insert(key(s)));
    out
}

/// Whether `p` is in the triangle `ps`, or on its sides, seen from above.
fn inside(ps: [P2; 3], p: P2) -> bool {
    let s = [(ps[0], ps[1]), (ps[1], ps[2]), (ps[2], ps[0])].map(|(u, w)| (w - u).cross(p - u));
    let slack = 1e-9 * (1.0 + (ps[1] - ps[0]).length() + (ps[2] - ps[0]).length());
    s.iter().all(|x| *x >= -slack) || s.iter().all(|x| *x <= slack)
}

/// The stretch of the edge from `a` to `b`, from 0 to 1 along it, that face `v` (of unit normal `n`, not upright) lies over
/// by more than [`UNDER`], seen from above; none where it hides none of it.
fn under(v: [P3; 3], n: P3, a: P3, b: P3) -> Option<(f64, f64)> {
    let flat = |p: P3| P2(p[0], p[1]);
    let (p, q) = (flat(a), flat(b));
    let d = q - p;
    let ps = v.map(flat);
    let area = (ps[1] - ps[0]).cross(ps[2] - ps[0]);
    if area.abs() < 1e-9 {
        return None;
    }
    let s = area.signum();
    // Where the edge is within the face seen from above: on the inner side of each of its three sides.
    let (mut t0, mut t1) = (0.0f64, 1.0f64);
    for (u, w) in [(ps[0], ps[1]), (ps[1], ps[2]), (ps[2], ps[0])] {
        let e = w - u;
        let (c0, c1) = (s * e.cross(p - u), s * e.cross(d));
        let slack = BESIDE * e.length();
        if c1.abs() < 1e-12 {
            if c0 < -slack {
                return None;
            }
            continue;
        }
        let t = (-slack - c0) / c1;
        if c1 > 0.0 {
            t0 = t0.max(t);
        } else {
            t1 = t1.min(t);
        }
    }
    if t1 - t0 <= 1e-9 {
        return None;
    }
    // Where the face is over it by more than UNDER: the face's height less the edge's, which goes as a line along it.
    let over = |x: P2, z: f64| v[0][2] - (n[0] * (x.0 - v[0][0]) + n[1] * (x.1 - v[0][1])) / n[2] - z;
    let (h0, h1) = (over(p, a[2]), over(q, b[2]));
    let slope = h1 - h0;
    if slope.abs() < 1e-12 {
        return (h0 > UNDER).then_some((t0, t1));
    }
    let t = (UNDER - h0) / slope;
    if slope > 0.0 {
        t0 = t0.max(t);
    } else {
        t1 = t1.min(t);
    }
    (t1 - t0 > 1e-9).then_some((t0, t1))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A box from `lo` to `hi`, two triangles to a face, as files bring them (the same corner written once for each face).
    fn cube(lo: P3, hi: P3) -> Vec<[P3; 3]> {
        let c = |i: usize| {
            [if i & 1 == 0 { lo[0] } else { hi[0] }, if i & 2 == 0 { lo[1] } else { hi[1] }, if i & 4 == 0 { lo[2] } else { hi[2] }]
        };
        let quads = [[0, 2, 3, 1], [4, 5, 7, 6], [0, 1, 5, 4], [2, 6, 7, 3], [0, 4, 6, 2], [1, 3, 7, 5]];
        quads.iter().flat_map(|q| [[c(q[0]), c(q[1]), c(q[2])], [c(q[0]), c(q[2]), c(q[3])]]).collect()
    }

    fn length(lines: &[[P2; 2]]) -> f64 {
        lines.iter().map(|[a, b]| a.dist(*b)).sum()
    }

    /// Whether some line runs from `a` to `b`, either way, to a twentieth of a millimetre.
    fn has(lines: &[[P2; 2]], a: P2, b: P2) -> bool {
        lines.iter().any(|[p, q]| (p.dist(a) < 0.05 && q.dist(b) < 0.05) || (p.dist(b) < 0.05 && q.dist(a) < 0.05))
    }

    #[test]
    fn a_box_draws_its_outline_and_not_the_diagonals_of_its_faces() {
        let lines = edges(&cube([0.0, 0.0, 0.0], [1000.0, 600.0, 900.0]));
        assert_eq!(lines.len(), 4, "{lines:?}");
        assert!((length(&lines) - 3200.0).abs() < 1e-6);
        assert!(has(&lines, P2(0.0, 0.0), P2(1000.0, 0.0)) && has(&lines, P2(1000.0, 600.0), P2(0.0, 600.0)));
    }

    #[test]
    fn what_a_higher_part_covers_is_hidden_and_the_rest_drawn() {
        // A table: its top over a leg, which nothing of shows.
        let mut table = cube([0.0, 0.0, 700.0], [1000.0, 1000.0, 750.0]);
        table.extend(cube([50.0, 50.0, 0.0], [100.0, 100.0, 700.0]));
        let lines = edges(&table);
        assert_eq!(lines.len(), 4, "the top's outline alone: {lines:?}");
        // A block on a counter, over its east edge: the counter's edge shows on either side of it.
        let mut counter = cube([0.0, 0.0, 0.0], [2000.0, 1000.0, 500.0]);
        counter.extend(cube([1500.0, 200.0, 500.0], [2500.0, 800.0, 900.0]));
        let lines = edges(&counter);
        assert!(has(&lines, P2(2000.0, 0.0), P2(2000.0, 200.0)) && has(&lines, P2(2000.0, 800.0), P2(2000.0, 1000.0)), "{lines:?}");
        assert!(!lines.iter().any(|[a, b]| a.0 == 2000.0 && b.0 == 2000.0 && a.1.max(b.1) > 200.5 && a.1.min(b.1) < 799.5), "{lines:?}");
        // The block's outline, whole, and the counter's other three sides.
        assert!((length(&lines) - (2.0 * 1000.0 + 2.0 * 600.0 + 2.0 * 2000.0 + 1000.0 + 400.0)).abs() < 0.1, "{lines:?}");
    }

    /// A basin turned about its upright axis: a flat rim from `r` to `rim` at `z`, a bowl down from it to a drain of
    /// `drain` mm, in `around` facets about and `down` rings, and its outer side down to the floor.
    fn basin(r: f64, rim: f64, z: f64, drain: f64, around: usize, down: usize) -> Vec<[P3; 3]> {
        let at = |radius: f64, k: usize, h: f64| {
            let (s, c) = (std::f64::consts::TAU * k as f64 / around as f64).sin_cos();
            [radius * c, radius * s, h]
        };
        let mut tris = Vec::new();
        let mut quad = |a: P3, b: P3, c: P3, d: P3| {
            tris.push([a, b, c]);
            tris.push([a, c, d]);
        };
        for k in 0..around {
            let j = k + 1;
            quad(at(r, k, z), at(rim, k, z), at(rim, j, z), at(r, j, z));
            quad(at(rim, k, z), at(rim, k, 0.0), at(rim, j, 0.0), at(rim, j, z));
            // The bowl: a quarter of an ellipse from the rim down to the drain, `down` rings.
            for i in 0..down {
                let ring = |i: usize| {
                    let a = std::f64::consts::FRAC_PI_2 * i as f64 / down as f64;
                    (drain + (r - drain) * a.cos(), z - 200.0 * a.sin())
                };
                let ((r0, z0), (r1, z1)) = (ring(i), ring(i + 1));
                quad(at(r0, k, z0), at(r0, j, z0), at(r1, j, z1), at(r1, k, z1));
            }
        }
        tris
    }

    #[test]
    fn a_basin_draws_its_rims_and_its_drain_and_none_of_the_facets_of_its_bowl() {
        let (around, down) = (48, 8);
        let lines = edges(&basin(200.0, 250.0, 850.0, 20.0, around, down));
        let radius = |p: P2| p.0.hypot(p.1);
        let on = |r: f64| lines.iter().filter(|[a, b]| (radius(*a) - r).abs() < 0.5 && (radius(*b) - r).abs() < 0.5).count();
        assert_eq!(on(250.0), around, "the outer rim");
        assert_eq!(on(200.0), around, "where the bowl falls from the rim");
        assert_eq!(on(20.0), around, "the drain");
        assert_eq!(lines.len(), 3 * around, "no line between the bowl's facets: {lines:?}");
    }

    #[test]
    fn a_surface_seen_edge_on_draws_its_line_once() {
        // A curtain: an upright sheet, its top and its bottom over each other.
        let sheet = vec![
            [[0.0, 0.0, 0.0], [2000.0, 0.0, 0.0], [2000.0, 0.0, 2500.0]],
            [[0.0, 0.0, 0.0], [2000.0, 0.0, 2500.0], [0.0, 0.0, 2500.0]],
        ];
        let lines = edges(&sheet);
        assert_eq!(lines.len(), 1, "{lines:?}");
        assert!(has(&lines, P2(0.0, 0.0), P2(2000.0, 0.0)));
    }

    #[test]
    fn a_seam_where_faces_of_one_surface_meet_without_sharing_their_corners_is_not_drawn() {
        // A square counter top: one triangle on one side of its diagonal, two meeting at the diagonal's middle on the other.
        let p = |x: f64, y: f64| [x, y, 900.0];
        let top = vec![
            [p(0.0, 0.0), p(100.0, 0.0), p(100.0, 100.0)],
            [p(0.0, 0.0), p(50.0, 50.0), p(0.0, 100.0)],
            [p(50.0, 50.0), p(100.0, 100.0), p(0.0, 100.0)],
        ];
        let lines = edges(&top);
        assert_eq!(lines.len(), 4, "its four sides, not the diagonal: {lines:?}");
    }

    #[test]
    fn a_fin_standing_in_a_surface_is_not_drawn_where_the_surface_goes_on_past_it() {
        // A counter top as an exporter may write one round a sink's hole: its faces meet the diagonal without sharing it, and
        // an upright fin stands along it, down to its underside.
        let mut tris = vec![
            [[0.0, 0.0, 20.0], [100.0, 0.0, 20.0], [100.0, 100.0, 20.0]],
            [[0.0, 0.0, 20.0], [50.0, 50.0, 20.0], [0.0, 100.0, 20.0]],
            [[50.0, 50.0, 20.0], [100.0, 100.0, 20.0], [0.0, 100.0, 20.0]],
        ];
        tris.extend([
            [[0.0, 0.0, 20.0], [100.0, 100.0, 20.0], [100.0, 100.0, 0.0]],
            [[0.0, 0.0, 20.0], [100.0, 100.0, 0.0], [0.0, 0.0, 0.0]],
        ]);
        let lines = edges(&tris);
        assert_eq!(lines.len(), 4, "the top's four sides: {lines:?}");
        assert!(!lines.iter().any(|[a, b]| (a.0 - a.1).abs() < 1e-6 && (b.0 - b.1).abs() < 1e-6), "nothing along the diagonal");
    }

    #[test]
    fn a_fold_drawn_goes_on_where_it_softens_for_a_stretch() {
        // A valley along x, its sides rising 18° then 13° and again: folds of 36° and 26° in turn.
        let rise = |i: usize| 100.0 * if i % 2 == 1 { 18f64 } else { 13f64 }.to_radians().tan();
        let (c, l, r) = (
            |i: usize| [100.0 * i as f64, 0.0, 0.0],
            |i: usize| [100.0 * i as f64, -100.0, rise(i.max(1))],
            |i: usize| [100.0 * i as f64, 100.0, rise(i.max(1))],
        );
        let mut tris = Vec::new();
        for i in 0..4 {
            tris.extend([[c(i), c(i + 1), l(i + 1)], [c(i), l(i + 1), l(i)], [c(i), r(i + 1), c(i + 1)], [c(i), r(i), r(i + 1)]]);
        }
        let lines = edges(&tris);
        let valley: f64 = lines.iter().filter(|[a, b]| a.1.abs() < 1e-6 && b.1.abs() < 1e-6).map(|[a, b]| a.dist(*b)).sum();
        assert!((valley - 400.0).abs() < 1e-6, "the whole valley: {valley} mm of it; {lines:?}");
    }

    #[test]
    fn nothing_usable_draws_nothing_and_what_was_worked_out_is_kept() {
        assert!(edges(&[]).is_empty());
        let bad = [[[f64::NAN, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]], [[0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [5.0, 5.0, 5.0]]];
        assert!(edges(&bad).is_empty());
        let b = cube([0.0, 0.0, 0.0], [10.0, 20.0, 30.0]);
        let (one, two) = (seen(&b), seen(&b));
        assert!(Arc::ptr_eq(&one, &two));
        assert_eq!(one.len(), 4);
    }
}
