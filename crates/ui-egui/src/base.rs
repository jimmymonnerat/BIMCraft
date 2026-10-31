//! The base a project is drawn over, set up as a designer sets it before drawing, on the Plumbing tab's Base panel: Scale by
//! Reference (two points of a known distance, then the distance typed: `scale_by_reference`); Flatten, where the Cut Planes
//! dialog says where each storey of the architect's model is cut and how far down it is seen, over a sketch of both
//! (`flatten_link`, the link kept for the 3D view); and Erase from Base, each click taking an object of the flattened base
//! away whole, a sofa or a rug (`cad_erase`).

use bimcraft_core::model::{Element, Id};
use bimcraft_i18n::msg;
use egui::{Color32, Context, Id as UiId, RichText, Stroke, Ui};
use serde_json::json;

use crate::lang::{t, tf};
use crate::popup::Modal;
use crate::style::pal;
use crate::{BimApp, Tool};

/// Cut Planes, as it stands: the link to flatten (of those the project has) and a row for each storey.
#[derive(Clone, Debug, PartialEq)]
pub struct CutPlanes {
    pub link: Id,
    pub links: Vec<(Id, String)>,
    pub rows: Vec<CutRow>,
    /// Why the last OK didn't go, said inside the dialog.
    pub error: Option<String>,
}

/// A storey in Cut Planes: whether it is flattened, where its plan is cut and down to where it is seen, in metres over its
/// level as typed (a comma or a point before the decimals, under its floor below 0).
#[derive(Clone, Debug, PartialEq)]
pub struct CutRow {
    pub level: Id,
    pub name: String,
    /// Its level's elevation and its height, mm.
    pub elevation: f64,
    pub height: f64,
    pub on: bool,
    pub cut: String,
    pub bottom: String,
}

/// A height over a level typed in metres, as mm: "1,68", "-0.42"; up to 100 m either way.
pub(crate) fn signed_metres(text: &str) -> Option<f64> {
    let m: f64 = text.trim().replace(',', ".").parse().ok()?;
    (m.is_finite() && m.abs() <= 100.0).then(|| (m * 1000.0).round())
}

/// Scale by Reference once both points are clicked: where, and the distance typed (metres).
#[derive(Clone, Debug, PartialEq)]
pub struct ScaleDialog {
    pub from: bimcraft_core::geom::P2,
    pub to: bimcraft_core::geom::P2,
    pub length: String,
    pub error: Option<String>,
}

impl BimApp {
    /// Flatten: Cut Planes, for the project's links and storeys, each storey cut at 60 % of its height and seen down to 15 %
    /// of it under its floor until it's said otherwise; or, with nothing linked, why not.
    pub(crate) fn open_cut_planes(&mut self) {
        let m = &self.session.model;
        let links: Vec<(Id, String)> = m
            .elements
            .iter()
            .filter_map(|(id, e)| match e {
                Element::Link(l) => Some((*id, l.name.clone())),
                _ => None,
            })
            .collect();
        let Some(&(link, _)) = links.first() else {
            self.status = crate::Status {
                text: t("There is no linked model to flatten: link the architect's model first.").to_string(),
                error: true,
            };
            return;
        };
        let mut levels: Vec<(Id, String, f64)> = m.levels().into_iter().map(|(id, l)| (id, l.name.clone(), l.elevation)).collect();
        levels.sort_by(|a, b| a.2.total_cmp(&b.2));
        let rows = (0..levels.len())
            .map(|i| {
                let (level, name, elevation) = levels[i].clone();
                let up = levels.get(i + 1).map(|n| n.2 - elevation);
                let down = i.checked_sub(1).and_then(|j| levels.get(j)).map(|d| elevation - d.2);
                let height = up.or(down).filter(|h| *h > 0.0).unwrap_or(3000.0);
                let cut = metres_text((bimcraft_core::flatten::CUT * height).round());
                let bottom = metres_text((bimcraft_core::flatten::BOTTOM * height).round());
                CutRow { level, name, elevation, height, on: true, cut, bottom }
            })
            .collect();
        self.cut_planes = Some(CutPlanes { link, links, rows, error: None });
    }

    pub(crate) fn cut_planes_window(&mut self, ctx: &Context) {
        let Some(mut d) = self.cut_planes.take() else { return };
        let (mut ok, mut cancel) = (false, false);
        let r = Modal::new(UiId::new("cut-planes"), t("Cut Planes")).width(860.0).show(ctx, |ui| {
            crate::popup::body(ui, |ui| {
                ui.add_space(4.0);
                ui.label(
                    RichText::new(t(
                        "Where each storey of the architect's model is cut, and down to where it is seen: what lies between becomes grey \
                         lines to draw over. The model stays for the 3D view.",
                    ))
                    .color(pal().muted),
                );
                ui.add_space(8.0);
                if d.links.len() > 1 {
                    ui.horizontal(|ui| {
                        ui.label(t("Model:"));
                        let shown = d.links.iter().find(|(id, _)| *id == d.link).map_or("", |(_, n)| n.as_str()).to_string();
                        egui::ComboBox::from_id_salt("cut-planes-link").selected_text(shown).show_ui(ui, |ui| {
                            for (id, name) in &d.links {
                                ui.selectable_value(&mut d.link, *id, name);
                            }
                        });
                    });
                    ui.add_space(6.0);
                }
                ui.horizontal_top(|ui| {
                    sketch(ui);
                    ui.add_space(12.0);
                    egui::Grid::new("cut-planes-grid").num_columns(8).spacing([10.0, 6.0]).show(ui, |ui| {
                        let heads = [
                            "",
                            msg!("Storey"),
                            msg!("Height (m)"),
                            msg!("Cut (m)"),
                            msg!("Bottom (m)"),
                            msg!("Elevation (m)"),
                            msg!("Cut at (m)"),
                            msg!("Seen down to (m)"),
                        ];
                        for head in heads {
                            ui.label(RichText::new(if head.is_empty() { String::new() } else { t(head).to_string() }).strong());
                        }
                        ui.end_row();
                        for row in d.rows.iter_mut().rev() {
                            ui.checkbox(&mut row.on, "");
                            ui.label(&row.name);
                            ui.label(RichText::new(metres_text(row.height)).color(pal().muted));
                            ui.add_sized([70.0, 22.0], egui::TextEdit::singleline(&mut row.cut));
                            ui.add_sized([70.0, 22.0], egui::TextEdit::singleline(&mut row.bottom));
                            let at =
                                |typed: &str| signed_metres(typed).map_or_else(|| "?".to_string(), |mm| metres_text(row.elevation + mm));
                            ui.label(RichText::new(metres_text(row.elevation)).color(pal().muted));
                            ui.label(RichText::new(at(&row.cut)).color(pal().muted));
                            ui.label(RichText::new(at(&row.bottom)).color(pal().muted));
                            ui.end_row();
                        }
                    });
                });
                if let Some(e) = &d.error {
                    ui.add_space(6.0);
                    ui.label(RichText::new(e).color(pal().error));
                }
            });
            crate::popup::footer(ui, |ui| {
                ui.horizontal(|ui| {
                    ok = ui.button(t("OK")).clicked();
                    cancel = ui.button(t("Cancel")).clicked();
                });
            });
        });
        if ok {
            let mut levels = Vec::new();
            for row in d.rows.iter().filter(|r| r.on) {
                let (Some(cut), Some(bottom)) = (signed_metres(&row.cut), signed_metres(&row.bottom)) else {
                    d.error = Some(tf("{}: the cut and the bottom are in metres over its level, as 1,68 and -0,42.", &[&row.name]));
                    self.cut_planes = Some(d);
                    return;
                };
                levels.push(json!({"level": row.level, "cut": cut, "bottom": bottom}));
            }
            if levels.is_empty() {
                d.error = Some(t("Tick a storey to flatten.").to_string());
                self.cut_planes = Some(d);
                return;
            }
            let Some(r) = self.run("flatten_link", json!({"id": d.link, "levels": levels, "keep_link": true})) else {
                // Said inside the dialog: a status box in the view's corner could lie over it.
                d.error = Some(std::mem::take(&mut self.status).text);
                self.cut_planes = Some(d);
                return;
            };
            let storeys = r["drawings"].as_array().map_or(0, Vec::len).to_string();
            self.status = crate::Status { text: tf("Flattened: {} storeys drawn in lines.", &[&storeys]), error: false };
            return;
        }
        if !(cancel || r.should_close()) {
            self.cut_planes = Some(d);
        }
    }

    /// Erase from Base: the object of the flattened base (a CAD import of the level) nearest `at`, within `reach` mm, taken
    /// away whole; or, with none there, said so.
    pub(crate) fn erase_from_base(&mut self, level: Id, at: bimcraft_core::geom::P2, reach: f64) {
        let m = &self.session.model;
        let imports: Vec<Id> =
            m.elements.iter().filter(|(_, e)| matches!(e, Element::CadImport(ci) if m.same(ci.level, level))).map(|(id, _)| *id).collect();
        let mut nearest: Option<(Id, f64)> = None;
        for id in imports {
            let Ok(r) = self.session.execute("cad_query", &json!({"id": id, "at": [at.0, at.1], "reach": reach})) else { continue };
            let d = r["distance"].as_f64().unwrap_or(f64::INFINITY);
            if r["found"] == true && nearest.is_none_or(|(_, n)| d < n) {
                nearest = Some((id, d));
            }
        }
        let Some((id, _)) = nearest else {
            self.status = crate::Status { text: t("Nothing of the base there: click on one of its lines.").to_string(), error: true };
            return;
        };
        if let Some(r) = self.run("cad_erase", json!({"id": id, "at": [at.0, at.1], "reach": reach})) {
            let (lines, layer) = (r["entities"].to_string(), r["layer"].as_str().unwrap_or("").to_string());
            self.status = crate::Status {
                text: tf("Erased from the base: {} lines of {}. Ctrl+Z brings them back.", &[&lines, &layer]),
                error: false,
            };
        }
    }

    /// The two points are clicked: the distance between them asked for, in metres, the measure typed in to start from.
    pub(crate) fn ask_scale(&mut self, from: bimcraft_core::geom::P2, to: bimcraft_core::geom::P2) {
        self.scale_dialog = Some(ScaleDialog { from, to, length: metres_text(from.dist(to)), error: None });
    }

    pub(crate) fn scale_window(&mut self, ctx: &Context) {
        let Some(mut d) = self.scale_dialog.take() else { return };
        let (mut ok, mut cancel) = (false, false);
        let r = Modal::new(UiId::new("scale-by-reference"), t("Scale by Reference")).width(420.0).show(ctx, |ui| {
            crate::popup::body(ui, |ui| {
                ui.add_space(4.0);
                let now = metres_text(d.from.dist(d.to));
                ui.label(RichText::new(tf("The two points are {} m apart now. How far apart are they?", &[&now])).color(pal().muted));
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.label(t("Distance (m):"));
                    let edit = ui.add_sized([120.0, 22.0], egui::TextEdit::singleline(&mut d.length));
                    if edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        ok = true;
                    }
                });
                if let Some(e) = &d.error {
                    ui.add_space(6.0);
                    ui.label(RichText::new(e).color(pal().error));
                }
            });
            crate::popup::footer(ui, |ui| {
                ui.horizontal(|ui| {
                    ok |= ui.button(t("OK")).clicked();
                    cancel = ui.button(t("Cancel")).clicked();
                });
            });
        });
        if ok {
            let Some(mm) = crate::newproject::metres(&d.length).or_else(|| small_metres(&d.length)) else {
                d.error = Some(t("The distance is in metres: 0.15 for a 15 cm wall.").to_string());
                self.scale_dialog = Some(d);
                return;
            };
            let p = json!({"from": [d.from.0, d.from.1], "to": [d.to.0, d.to.1], "length": mm});
            let Some(reply) = self.run("scale_by_reference", p) else {
                // Said inside the form: a status box in the view's corner could lie over it.
                d.error = Some(std::mem::take(&mut self.status).text);
                self.scale_dialog = Some(d);
                return;
            };
            let factor = reply["factor"].as_f64().unwrap_or(1.0);
            let text = if (factor - 1.0).abs() < 1e-9 {
                t("The base is at scale already.").to_string()
            } else {
                tf("The base is scaled by {}.", &[&number_text(factor, 4)])
            };
            self.status = crate::Status { text, error: false };
            self.tool = Tool::ScaleRef(None);
            return;
        }
        if !(cancel || r.should_close()) {
            self.scale_dialog = Some(d);
        }
    }
}

/// The sketch beside Cut Planes' table: a storey in section, its level, the cut plane over it and the bottom of what is seen
/// under its floor, drawn here.
fn sketch(ui: &mut Ui) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(230.0, 190.0), egui::Sense::hover());
    let p = ui.painter_at(rect);
    let ink = pal().text;
    let (cut, bottom) = (Color32::from_rgb(214, 64, 64), Color32::from_rgb(56, 120, 214));
    let x = |f: f32| rect.left() + f * rect.width();
    let y = |f: f32| rect.top() + f * rect.height();
    // The storey: its floor, a wall up to the floor over it, a basin and a WC against the wall.
    let (floor, over) = (y(0.78), y(0.12));
    p.line_segment([egui::pos2(x(0.05), floor), egui::pos2(x(0.62), floor)], Stroke::new(2.0, ink));
    p.line_segment([egui::pos2(x(0.05), over), egui::pos2(x(0.62), over)], Stroke::new(2.0, ink));
    p.rect_stroke(
        egui::Rect::from_min_max(egui::pos2(x(0.05), over), egui::pos2(x(0.09), floor)),
        0.0,
        Stroke::new(1.0, ink),
        egui::StrokeKind::Inside,
    );
    p.rect_stroke(
        egui::Rect::from_min_max(egui::pos2(x(0.09), y(0.54)), egui::pos2(x(0.22), y(0.58))),
        0.0,
        Stroke::new(1.0, ink),
        egui::StrokeKind::Inside,
    );
    p.rect_stroke(
        egui::Rect::from_min_max(egui::pos2(x(0.30), y(0.62)), egui::pos2(x(0.40), floor)),
        3.0,
        Stroke::new(1.0, ink),
        egui::StrokeKind::Inside,
    );
    // The cut plane, and the bottom under the floor, dashed, each with what it is.
    let dashed = |at: f32, c: Color32| {
        p.extend(egui::Shape::dashed_line(&[egui::pos2(x(0.02), at), egui::pos2(x(0.66), at)], Stroke::new(1.5, c), 6.0, 4.0))
    };
    let (cut_at, bottom_at) = (y(0.36), y(0.9));
    dashed(cut_at, cut);
    dashed(bottom_at, bottom);
    let font = egui::FontId::proportional(11.0);
    let label = |at: f32, text: &str, c: Color32| {
        p.text(egui::pos2(x(0.68), at), egui::Align2::LEFT_CENTER, text, font.clone(), c);
    };
    label(cut_at, t("Cut plane"), cut);
    label(floor, t("Level"), ink);
    label(bottom_at, t("Bottom"), bottom);
    // The cut and the bottom measured from the level.
    let arrow = |to: f32, c: Color32| p.arrow(egui::pos2(x(0.5), floor), egui::vec2(0.0, to - floor), Stroke::new(1.2, c));
    arrow(cut_at, cut);
    arrow(bottom_at, bottom);
}

/// A distance under 0.1 m (the smallest a height takes), as a wall's thickness is: 0.015 for 15 mm.
fn small_metres(text: &str) -> Option<f64> {
    let m: f64 = text.trim().replace(',', ".").parse().ok()?;
    (m.is_finite() && m > 0.0 && m < 0.1).then_some(m * 1000.0)
}

/// A number as it is read in Portuguese and in English alike: up to `places` decimals, none trailing, a comma before them.
fn number_text(x: f64, places: usize) -> String {
    let s = format!("{x:.places$}");
    let s = if s.contains('.') { s.trim_end_matches('0').trim_end_matches('.').to_string() } else { s };
    s.replace('.', ",")
}

/// A distance in mm as metres to type: "0,015", "3,06".
pub(crate) fn metres_text(mm: f64) -> String {
    number_text(mm / 1000.0, 3)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cut_or_a_bottom_is_read_in_metres_either_side_of_the_level() {
        assert_eq!(signed_metres("1,68"), Some(1680.0));
        assert_eq!(signed_metres(" -0.42 "), Some(-420.0));
        assert_eq!(signed_metres("0"), Some(0.0));
        assert_eq!(signed_metres("high"), None);
        assert_eq!(signed_metres("1e9"), None);
        assert_eq!(metres_text(-420.0), "-0,42");
    }

    #[test]
    fn distances_read_in_metres_with_a_comma() {
        assert_eq!(metres_text(15.0), "0,015");
        assert_eq!(metres_text(3060.0), "3,06");
        assert_eq!(metres_text(5000.0), "5");
        assert_eq!(small_metres("0,015"), Some(15.0));
        assert_eq!(small_metres("0.15"), None, "from 0.1 up, a distance reads as a height does");
        assert_eq!(number_text(10.0, 4), "10");
    }
}
