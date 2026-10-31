//! The base a project is drawn over, set up as a designer sets it before drawing: Scale by Reference (two points of a known
//! distance, then the distance typed), on the Plumbing tab's Base panel. The command is `scale_by_reference`.

use egui::{Context, Id as UiId, RichText};
use serde_json::json;

use crate::lang::{t, tf};
use crate::popup::Modal;
use crate::style::pal;
use crate::{BimApp, Tool};

/// Scale by Reference once both points are clicked: where, and the distance typed (metres).
#[derive(Clone, Debug, PartialEq)]
pub struct ScaleDialog {
    pub from: bimcraft_core::geom::P2,
    pub to: bimcraft_core::geom::P2,
    pub length: String,
    pub error: Option<String>,
}

impl BimApp {
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

/// A distance under 0.1 m (the smallest a height takes), as a wall's thickness is: 0.015 for 15 mm.
fn small_metres(text: &str) -> Option<f64> {
    let m: f64 = text.trim().replace(',', ".").parse().ok()?;
    (m.is_finite() && m > 0.0 && m < 0.1).then(|| m * 1000.0)
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
    fn distances_read_in_metres_with_a_comma() {
        assert_eq!(metres_text(15.0), "0,015");
        assert_eq!(metres_text(3060.0), "3,06");
        assert_eq!(metres_text(5000.0), "5");
        assert_eq!(small_metres("0,015"), Some(15.0));
        assert_eq!(small_metres("0.15"), None, "from 0.1 up, a distance reads as a height does");
        assert_eq!(number_text(10.0, 4), "10");
    }
}
