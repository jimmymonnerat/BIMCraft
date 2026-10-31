//! Flattening a linked model: the architect's IFC cut at each storey as its plan cuts it (where the cut plane and the bottom of
//! the view are asked for, else at 60 % of the storey's height and down to 15 % of it under its floor, as the designer's
//! old program cuts a storey), and its lines kept as a drawing of the project's own (a DXF in millimetres, placed on that
//! level as an imported one is), so the hydraulics are drawn over light, 2D lines that no longer need the model. The
//! `flatten_link` command makes them. Each element's lines go on a layer named for what it is (a component's, for what its
//! family is: furniture, plumbing…) and are one object of the drawing, a block placed once whose handle is the element's,
//! so a sofa is erased whole (`cad_erase`) and stays erased when the storey is flattened again; they are joined into
//! polylines, and a ring that is a circle (a drain) is written as one, so its centre is snapped to.

use std::collections::{BTreeMap, HashMap};

use crate::geom::P2;
use crate::model::{Element, Id, Model};
use crate::render::{self, Part, Prim, View};

/// Pixels per millimetre the cut is drawn at: one, so a line's ends are as exact as `f32` keeps millimetres.
const PX_PER_MM: f64 = 1.0;
/// Room round the model in the drawing, mm: so nothing at its edge is cut off.
const MARGIN: f64 = 2000.0;
/// The widest a drawing is drawn, px (and so mm): a model wider than 50 km is no building.
const MAX_PX: f64 = 50_000_000.0;
/// Where a storey is cut unless it's said, and down to where it's seen, as parts of its height: 1,68 m and 0,42 m under its
/// floor for a storey 2,80 m high.
pub const CUT: f64 = 0.6;
pub const BOTTOM: f64 = -0.15;

/// A line of a flattened plan: the layer it goes on (what its element is), the object it is of (its element's handle), and
/// its ends, mm.
#[derive(Clone, Debug, PartialEq)]
pub struct Stroke {
    pub layer: String,
    pub object: String,
    pub a: P2,
    pub b: P2,
}

/// The lines `m`'s plan of `level` draws of what `keep` takes (a linked element), each with its element's layer and handle:
/// strokes only, in millimetres, a closed line's last edge too. What is cut and what is seen go; fills, the hatching of a cut
/// (thousands of strokes a base needs none of), lettering, pictures and what nothing owns (annotation) stay out.
pub fn plan_lines(m: &Model, level: Id, keep: impl Fn(Id) -> bool) -> Vec<Stroke> {
    let Some((lo, hi)) = m.model_box() else { return Vec::new() };
    let (w, h) = ((hi[0] - lo[0] + 2.0 * MARGIN) * PX_PER_MM, (hi[1] - lo[1] + 2.0 * MARGIN) * PX_PER_MM);
    if !(w.is_finite() && h.is_finite()) || w > MAX_PX || h > MAX_PX {
        return Vec::new();
    }
    let view =
        View { scale: PX_PER_MM, cx: (lo[0] + hi[0]) / 2.0, cy: (lo[1] + hi[1]) / 2.0, width: w.ceil() as u32, height: h.ceil() as u32 };
    let d = render::draw(m, level, view);
    let mut out = Vec::new();
    let mut named: HashMap<Id, (String, String)> = HashMap::new();
    for (i, (prim, owner)) in d.prims.iter().zip(&d.owners).enumerate() {
        let (Prim::Line { pts, closed, .. }, Some(id)) = (prim, owner) else { continue };
        if !keep(*id) || !matches!(d.parts.get(i), Some(Part::Cut | Part::Projection) | None) {
            continue;
        }
        let (layer, object) = named.entry(*id).or_insert_with(|| (layer_of(m, *id), handle(m, *id))).clone();
        let mm: Vec<P2> = pts.iter().map(|p| round(view.mm(*p))).collect();
        let closing = (*closed && mm.len() > 2).then(|| (mm[mm.len() - 1], mm[0]));
        for (a, b) in mm.windows(2).map(|w| (w[0], w[1])).chain(closing) {
            if a.dist(b) > 0.01 {
                out.push(Stroke { layer: layer.clone(), object: object.clone(), a, b });
            }
        }
    }
    out
}

/// The layer an element's lines go on: what it is (`wall`, `door`…); a component's, what its family is (`furniture`,
/// `plumbing`…).
fn layer_of(m: &Model, id: Id) -> String {
    match m.elements.get(id) {
        Some(Element::Component(c)) => m
            .families
            .get(&c.family)
            .and_then(|f| serde_json::to_value(f.category).ok())
            .and_then(|v| v.as_str().map(str::to_string))
            .unwrap_or_else(|| "component".into()),
        Some(e) => e.category().to_string(),
        None => "other".into(),
    }
}

/// An element's handle in a flattened drawing: its GUID folded to 64 bits, in hex as a DXF writes handles; the same each
/// time the storey is flattened, so what was erased of it stays erased.
fn handle(m: &Model, id: Id) -> String {
    let g = m.elements.guid(id).map_or(u128::from(id), |g| g.as_u128());
    format!("{:X}", (g as u64) ^ ((g >> 64) as u64))
}

/// The ACI colour every layer of a flattened drawing has: a dark grey.
const GREY: u32 = 251;

/// To a hundredth of a millimetre: what `f32` pixels leave of a line's end.
fn round(p: P2) -> P2 {
    P2((p.0 * 100.0).round() / 100.0, (p.1 * 100.0).round() / 100.0)
}

/// `strokes` as an ASCII DXF in millimetres: a grey layer for each kind of element, and each element a block placed once,
/// its INSERT's handle the element's. Its lines are joined into polylines where two meet end to end, and a ring that is a
/// circle is written as one.
pub fn dxf(strokes: &[Stroke]) -> String {
    let mut layers: Vec<&str> = strokes.iter().map(|s| s.layer.as_str()).collect();
    layers.sort_unstable();
    layers.dedup();
    let mut objects: BTreeMap<&str, (&str, Vec<(P2, P2)>)> = BTreeMap::new();
    for s in strokes {
        objects.entry(s.object.as_str()).or_insert_with(|| (s.layer.as_str(), Vec::new())).1.push((s.a, s.b));
    }
    let mut out = String::from("0\nSECTION\n2\nHEADER\n9\n$INSUNITS\n70\n4\n0\nENDSEC\n0\nSECTION\n2\nTABLES\n0\nTABLE\n2\nLAYER\n");
    for l in &layers {
        // Grey, as a base is: what is drawn over it stands out.
        out += &format!("0\nLAYER\n2\n{l}\n70\n0\n62\n{GREY}\n6\nContinuous\n");
    }
    out += "0\nENDTAB\n0\nENDSEC\n0\nSECTION\n2\nBLOCKS\n";
    for (object, (_, segs)) in &objects {
        out += &format!("0\nBLOCK\n8\n0\n2\nE{object}\n70\n0\n10\n0\n20\n0\n");
        for (pts, closed) in chains(segs) {
            match circle(&pts).filter(|_| closed) {
                Some((c, r)) => out += &format!("0\nCIRCLE\n8\n0\n10\n{}\n20\n{}\n40\n{}\n", c.0, c.1, r),
                None => {
                    out += &format!("0\nLWPOLYLINE\n8\n0\n90\n{}\n70\n{}\n", pts.len(), u8::from(closed));
                    for p in &pts {
                        out += &format!("10\n{}\n20\n{}\n", p.0, p.1);
                    }
                }
            }
        }
        out += "0\nENDBLK\n8\n0\n";
    }
    out += "0\nENDSEC\n0\nSECTION\n2\nENTITIES\n";
    for (object, (layer, _)) in &objects {
        out += &format!("0\nINSERT\n5\n{object}\n8\n{layer}\n2\nE{object}\n10\n0\n20\n0\n");
    }
    out += "0\nENDSEC\n0\nEOF\n";
    out
}

/// `segs` joined end to end where two of them meet and no third: the runs that end somewhere, then the rings (`true`).
fn chains(segs: &[(P2, P2)]) -> Vec<(Vec<P2>, bool)> {
    let key = |p: P2| ((p.0 * 100.0).round() as i64, (p.1 * 100.0).round() as i64);
    let mut at: BTreeMap<(i64, i64), Vec<usize>> = BTreeMap::new();
    for (i, (a, b)) in segs.iter().enumerate() {
        at.entry(key(*a)).or_default().push(i);
        at.entry(key(*b)).or_default().push(i);
    }
    let mut used = vec![false; segs.len()];
    let run = |first: usize, start: P2, used: &mut Vec<bool>| {
        let (mut pts, mut seg, mut here) = (vec![start], first, start);
        loop {
            used[seg] = true;
            let (a, b) = segs[seg];
            here = if key(a) == key(here) { b } else { a };
            pts.push(here);
            match at.get(&key(here)).map(Vec::as_slice) {
                Some([x, y]) if !used[*x] || !used[*y] => seg = if used[*x] { *y } else { *x },
                _ => break,
            }
        }
        pts
    };
    let mut out = Vec::new();
    for (k, here) in &at {
        if here.len() == 2 {
            continue;
        }
        for &s in here {
            if !used[s] {
                let (a, b) = segs[s];
                let start = if key(a) == *k { a } else { b };
                out.push((run(s, start, &mut used), false));
            }
        }
    }
    for s in 0..segs.len() {
        if !used[s] {
            let mut pts = run(s, segs[s].0, &mut used);
            let closed = pts.len() > 3 && pts.first().map(|p| key(*p)) == pts.last().map(|p| key(*p));
            if closed {
                pts.pop();
            }
            out.push((pts, closed));
        }
    }
    out
}

/// The circle a ring of points is, drawn as many short sides: its centre and radius, where there are a dozen corners or more,
/// all on one circle and none more than an eighth of a turn from the next round it.
fn circle(ring: &[P2]) -> Option<(P2, f64)> {
    if ring.len() < 12 {
        return None;
    }
    let n = ring.len() as f64;
    let c = ring.iter().fold(P2(0.0, 0.0), |s, p| s + *p) * (1.0 / n);
    let r = ring.iter().map(|p| p.dist(c)).sum::<f64>() / n;
    if !(r > 0.0) || ring.iter().any(|p| (p.dist(c) - r).abs() > 0.002 * r + 0.05) {
        return None;
    }
    let mut turns: Vec<f64> = ring.iter().map(|p| (p.1 - c.1).atan2(p.0 - c.0)).collect();
    turns.sort_by(f64::total_cmp);
    let gaps = turns.windows(2).map(|w| w[1] - w[0]).chain(turns.first().zip(turns.last()).map(|(a, b)| a + std::f64::consts::TAU - b));
    gaps.fold(0.0, f64::max).lt(&std::f64::consts::FRAC_PI_4).then_some((round(c), (r * 100.0).round() / 100.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stroke(layer: &str, object: &str, a: P2, b: P2) -> Stroke {
        Stroke { layer: layer.into(), object: object.into(), a, b }
    }

    #[test]
    fn lines_go_to_a_dxf_in_millimetres_each_element_an_object_on_a_layer_for_what_it_is() {
        let lines = vec![
            stroke("wall", "1A", P2(0.0, 0.0), P2(6000.0, 0.0)),
            stroke("wall", "1A", P2(6000.0, 0.0), P2(6000.0, 150.0)),
            stroke("door", "2B", P2(1.5, 2.25), P2(3.0, 4.0)),
        ];
        let read = crate::dxf::read_text(&dxf(&lines)).unwrap();
        assert_eq!(read.scale, Some(1.0), "millimetres");
        let names: Vec<&str> = read.layers.iter().map(|l| l.name.as_str()).collect();
        assert!(names.contains(&"wall") && names.contains(&"door"), "{names:?}");
        let found: Vec<(&str, &str, &crate::dxf::Shape)> =
            read.entities.iter().map(|e| (e.layer.as_str(), e.object.as_str(), &e.shape)).collect();
        let joined = crate::dxf::Shape::Polyline { pts: vec![P2(0.0, 0.0), P2(6000.0, 0.0), P2(6000.0, 150.0)], closed: false };
        let door = crate::dxf::Shape::Polyline { pts: vec![P2(1.5, 2.25), P2(3.0, 4.0)], closed: false };
        assert_eq!(found, [("wall", "1A", &joined), ("door", "2B", &door)]);
    }

    #[test]
    fn a_ring_of_short_sides_round_a_point_is_written_as_the_circle_it_is() {
        let ring: Vec<P2> = (0..24)
            .map(|k| {
                let (s, c) = (std::f64::consts::TAU * f64::from(k) / 24.0).sin_cos();
                P2(1000.0 + 20.0 * c, 500.0 + 20.0 * s)
            })
            .collect();
        let mut lines: Vec<Stroke> = (0..24).map(|k| stroke("plumbing", "3C", ring[k], ring[(k + 1) % 24])).collect();
        // A square and a half-round (a D) are no circles: their corners leave too wide a gap round.
        let square = [P2(0.0, 0.0), P2(100.0, 0.0), P2(100.0, 100.0), P2(0.0, 100.0)];
        lines.extend((0..4).map(|k| stroke("furniture", "4D", square[k], square[(k + 1) % 4])));
        let read = crate::dxf::read_text(&dxf(&lines)).unwrap();
        let circles: Vec<&crate::dxf::Shape> =
            read.entities.iter().map(|e| &e.shape).filter(|s| matches!(s, crate::dxf::Shape::Circle { .. })).collect();
        assert_eq!(circles, [&crate::dxf::Shape::Circle { c: P2(1000.0, 500.0), r: 20.0 }]);
        let half: Vec<P2> = (0..=12)
            .map(|k| {
                let (s, c) = (std::f64::consts::PI * f64::from(k) / 12.0).sin_cos();
                P2(50.0 * c, 50.0 * s)
            })
            .collect();
        assert_eq!(circle(&half), None);
        assert!(read.entities.iter().any(|e| matches!(&e.shape, crate::dxf::Shape::Polyline { pts, closed: true } if pts.len() == 4)));
    }

    /// A box from `lo` to `hi` as a file brings one: two triangles to a face.
    fn cube(lo: [f64; 3], hi: [f64; 3]) -> Vec<[[f64; 3]; 3]> {
        let c = |i: usize| {
            [if i & 1 == 0 { lo[0] } else { hi[0] }, if i & 2 == 0 { lo[1] } else { hi[1] }, if i & 4 == 0 { lo[2] } else { hi[2] }]
        };
        let quads = [[0, 2, 3, 1], [4, 5, 7, 6], [0, 1, 5, 4], [2, 6, 7, 3], [0, 4, 6, 2], [1, 3, 7, 5]];
        quads.iter().flat_map(|q| [[c(q[0]), c(q[1]), c(q[2])], [c(q[0]), c(q[2]), c(q[3])]]).collect()
    }

    #[test]
    fn a_form_a_file_brings_as_triangles_is_drawn_as_seen_from_above_not_as_the_hull_round_it() {
        // An L-shaped counter as an IFC brings it: two boxes of triangles in one form.
        let mut s = crate::engine::Session::new();
        let mut tris = cube([0.0, 0.0, 0.0], [2000.0, 600.0, 900.0]);
        tris.extend(cube([0.0, 600.0, 0.0], [600.0, 2000.0, 900.0]));
        let family = serde_json::json!({"name": "Bancada", "types": {"Bancada": {}}, "forms": [{"mesh": tris}]});
        s.execute("family", &family).unwrap();
        s.execute("component", &serde_json::json!({"family": "Bancada", "at": [0, 0]})).unwrap();
        let level = s.model.levels()[0].0;
        let lines = plan_lines(&s.model, level, |_| true);
        let near = |p: P2, reach: f64| lines.iter().any(|s| crate::geom::dist_to_segment(p, s.a, s.b) < reach);
        // The hull round an L cuts across its corner, from (2000, 600) to (600, 2000): nothing is drawn there.
        assert!(!near(P2(1300.0, 1300.0), 50.0), "{lines:?}");
        // Its inner corner is.
        assert!(near(P2(1300.0, 600.0), 1.0) && near(P2(600.0, 1300.0), 1.0), "{lines:?}");
    }
}
