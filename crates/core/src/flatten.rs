//! Flattening a linked model: the architect's IFC cut at each storey as its plan cuts it, and its lines kept as a drawing of the
//! project's own (a DXF in millimetres, placed on that level as an imported one is), so the hydraulics are drawn over light, 2D
//! lines that no longer need the model. The `flatten_link` command makes them; each element's lines go on a layer named for
//! its category.

use crate::geom::P2;
use crate::model::{Id, Model};
use crate::render::{self, Prim, View};

/// Pixels per millimetre the cut is drawn at: one, so a line's ends are as exact as `f32` keeps millimetres.
const PX_PER_MM: f64 = 1.0;
/// Room round the model in the drawing, mm: so nothing at its edge is cut off.
const MARGIN: f64 = 2000.0;
/// The widest a drawing is drawn, px (and so mm): a model wider than 50 km is no building.
const MAX_PX: f64 = 50_000_000.0;

/// The lines `m`'s plan of `level` draws of what `keep` takes (a linked element), each with the category of its element:
/// strokes only, in millimetres, a closed line's last edge too. Fills, pictures and what nothing owns (annotation) stay out.
pub fn plan_lines(m: &Model, level: Id, keep: impl Fn(Id) -> bool) -> Vec<(String, P2, P2)> {
    let Some((lo, hi)) = m.model_box() else { return Vec::new() };
    let (w, h) = ((hi[0] - lo[0] + 2.0 * MARGIN) * PX_PER_MM, (hi[1] - lo[1] + 2.0 * MARGIN) * PX_PER_MM);
    if !(w.is_finite() && h.is_finite()) || w > MAX_PX || h > MAX_PX {
        return Vec::new();
    }
    let view =
        View { scale: PX_PER_MM, cx: (lo[0] + hi[0]) / 2.0, cy: (lo[1] + hi[1]) / 2.0, width: w.ceil() as u32, height: h.ceil() as u32 };
    let d = render::draw(m, level, view);
    let mut out = Vec::new();
    for (prim, owner) in d.prims.iter().zip(&d.owners) {
        let (Prim::Line { pts, closed, .. }, Some(id)) = (prim, owner) else { continue };
        if !keep(*id) {
            continue;
        }
        let category = m.elements.get(*id).map_or("other", |e| e.category()).to_string();
        let mm: Vec<P2> = pts.iter().map(|p| round(view.mm(*p))).collect();
        let closing = (*closed && mm.len() > 2).then(|| (mm[mm.len() - 1], mm[0]));
        for (a, b) in mm.windows(2).map(|w| (w[0], w[1])).chain(closing) {
            if a.dist(b) > 0.01 {
                out.push((category.clone(), a, b));
            }
        }
    }
    out
}

/// To a hundredth of a millimetre: what `f32` pixels leave of a line's end.
fn round(p: P2) -> P2 {
    P2((p.0 * 100.0).round() / 100.0, (p.1 * 100.0).round() / 100.0)
}

/// `lines` as an ASCII DXF in millimetres: a layer for each category, a LINE for each.
pub fn dxf(lines: &[(String, P2, P2)]) -> String {
    let mut layers: Vec<&str> = lines.iter().map(|(l, ..)| l.as_str()).collect();
    layers.sort_unstable();
    layers.dedup();
    let mut out = String::from("0\nSECTION\n2\nHEADER\n9\n$INSUNITS\n70\n4\n0\nENDSEC\n0\nSECTION\n2\nTABLES\n0\nTABLE\n2\nLAYER\n");
    for (i, l) in layers.iter().enumerate() {
        // Colours 1 to 7, a layer each, so they tell apart in the view.
        out += &format!("0\nLAYER\n2\n{l}\n70\n0\n62\n{}\n6\nContinuous\n", i % 7 + 1);
    }
    out += "0\nENDTAB\n0\nENDSEC\n0\nSECTION\n2\nENTITIES\n";
    for (l, a, b) in lines {
        out += &format!("0\nLINE\n8\n{l}\n10\n{}\n20\n{}\n11\n{}\n21\n{}\n", a.0, a.1, b.0, b.1);
    }
    out += "0\nENDSEC\n0\nEOF\n";
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_go_to_a_dxf_in_millimetres_on_a_layer_for_each_category() {
        let lines = vec![("wall".to_string(), P2(0.0, 0.0), P2(6000.0, 0.0)), ("door".to_string(), P2(1.5, 2.25), P2(3.0, 4.0))];
        let read = crate::dxf::read_text(&dxf(&lines)).unwrap();
        assert_eq!(read.scale, Some(1.0), "millimetres");
        let names: Vec<&str> = read.layers.iter().map(|l| l.name.as_str()).collect();
        assert!(names.contains(&"wall") && names.contains(&"door"), "{names:?}");
        let shapes: Vec<&crate::dxf::Shape> = read.entities.iter().map(|e| &e.shape).collect();
        assert_eq!(
            shapes,
            [&crate::dxf::Shape::Line(P2(0.0, 0.0), P2(6000.0, 0.0)), &crate::dxf::Shape::Line(P2(1.5, 2.25), P2(3.0, 4.0))]
        );
    }
}
