//! Create Project: the start page's form for a project made the way a designer starts one, its name, the folder it is saved in
//! and its storeys (each one's name and height), the designer's own and not the architect's: a house of one storey starts as
//! the ground floor 2,80 m high and the roof level over it, its pipes under the floor and on the roof. The command is `new`
//! with `heights` and `names`, then `save` in that folder; the architect's model chosen is linked for the 3D view, out of
//! the plans until they are flattened (`link` with `in_plans: false`).

use std::path::PathBuf;

use egui::{Context, DragValue, Id as UiId, RichText, ScrollArea};
use serde_json::{Value, json};

use crate::lang::{t, tf};
use crate::popup::Modal;
use crate::style::pal;
use crate::{BimApp, Pending};

/// The most storeys the form makes, as the command takes.
const MAX_STOREYS: usize = 200;

/// A storey as typed: its level's name and its height in metres (a comma or a point before the decimals).
#[derive(Clone, Debug, PartialEq)]
pub struct Storey {
    pub name: String,
    pub height: String,
}

/// The form as it stands.
#[derive(Clone, Debug, PartialEq)]
pub struct CreateProject {
    pub name: String,
    pub folder: Option<PathBuf>,
    pub storeys: Vec<Storey>,
    /// The level over the last storey (a roof, a cover).
    pub top: String,
    /// The architect's model (an IFC) the project links as its base: how many storeys it has, and where, is the architect's.
    pub architecture: Option<PathBuf>,
    /// Why the last Create didn't go, said inside the form.
    pub error: Option<String>,
}

/// What Create hands on once the project behind is safe: the `new` command's parameters and where to save.
#[derive(Clone, Debug, PartialEq)]
pub struct Creating {
    pub(crate) params: Value,
    pub(crate) path: PathBuf,
    /// The architect's model to link, as the link keeps it: relative to the project's folder when it is in it.
    pub(crate) link: Option<String>,
}

/// A height typed in metres, as mm: "3", "2,88", "3.06", "0,05" (an architect's levels may be 5 cm apart).
pub(crate) fn metres(text: &str) -> Option<f64> {
    let m: f64 = text.trim().replace(',', ".").parse().ok()?;
    (m.is_finite() && (0.001..=100.0).contains(&m)).then(|| (m * 1000.0).round())
}

/// The words a new project's levels are named with, in its language: the first storey and the level over the last.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Words<'a> {
    pub ground: &'a str,
    pub roof: &'a str,
}

impl Words<'static> {
    /// As the project's language `l` says them.
    pub(crate) fn of(l: bimcraft_core::language::Language) -> Words<'static> {
        use bimcraft_core::language::{GROUND_FLOOR, ROOF_LEVEL};
        Words { ground: l.word(GROUND_FLOOR), roof: l.word(ROOF_LEVEL) }
    }
}

impl CreateProject {
    /// One storey 2,80 m high, the ground floor, and the roof level over it, as `w` names them.
    pub(crate) fn new(w: Words) -> CreateProject {
        CreateProject {
            name: String::new(),
            folder: None,
            storeys: vec![Storey { name: w.ground.to_string(), height: "2,80".into() }],
            top: w.roof.to_string(),
            architecture: None,
            error: None,
        }
    }

    /// As many storeys as `count`: new ones named after the last and as high, the top level renamed to follow while it has the
    /// name it was given.
    pub(crate) fn resize(&mut self, count: usize, level: &str) {
        let count = count.clamp(1, MAX_STOREYS);
        let default_top = format!("{level} {}", self.storeys.len() + 1);
        let height = self.storeys.last().map_or_else(|| "3,00".to_string(), |s| s.height.clone());
        while self.storeys.len() < count {
            let n = self.storeys.len() + 1;
            self.storeys.push(Storey { name: format!("{level} {n}"), height: height.clone() });
        }
        self.storeys.truncate(count);
        if self.top == default_top || self.top.trim().is_empty() {
            self.top = format!("{level} {}", count + 1);
        }
    }

    /// The command's parameters and the file to save, or what to fix first.
    pub(crate) fn creating(&self, language: &str) -> Result<Creating, String> {
        let name = self.name.trim();
        if name.is_empty() {
            return Err(t("Give the project a name.").to_string());
        }
        if name.contains(['/', '\\', ':', '*', '?', '"', '<', '>', '|']) {
            return Err(t("A project's name can't have / \\ : * ? \" < > or |.").to_string());
        }
        let Some(folder) = &self.folder else { return Err(t("Choose the folder to save it in.").to_string()) };
        let mut heights = Vec::new();
        for s in &self.storeys {
            let h = metres(&s.height).ok_or_else(|| tf("{}: the height is in metres, from 0.001 to 100", &[&s.name]))?;
            heights.push(h);
        }
        let mut names: Vec<String> = self.storeys.iter().map(|s| s.name.trim().to_string()).collect();
        names.push(self.top.trim().to_string());
        if names.iter().any(String::is_empty) {
            return Err(t("Every level needs a name.").to_string());
        }
        if names.iter().collect::<std::collections::BTreeSet<_>>().len() != names.len() {
            return Err(t("Two levels can't share a name.").to_string());
        }
        let path = folder.join(format!("{name}.bimcraft"));
        if path.exists() {
            return Err(tf("{} is there already: choose another name or folder.", &[&path.display().to_string()]));
        }
        let params = json!({"name": name, "language": language, "heights": heights, "names": names});
        let link = self.architecture.as_ref().map(|a| a.strip_prefix(folder).unwrap_or(a).to_string_lossy().into_owned());
        Ok(Creating { params, path, link })
    }

    /// The architect's model to link, `file`: the storeys stay as they are, the designer's, however many the architect
    /// drew; the project's name follows the file's while none is typed.
    pub(crate) fn follow(&mut self, file: &std::path::Path) {
        self.architecture = Some(file.to_path_buf());
        if self.name.trim().is_empty() {
            self.name = file.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        }
        self.error = None;
    }

    /// Each level's elevation as the heights typed put it, in metres, from the ground up and the top's last; none past a
    /// height that isn't one.
    pub(crate) fn elevations(&self) -> Vec<Option<String>> {
        let mut at = Some(0.0);
        let mut out = Vec::new();
        for s in &self.storeys {
            out.push(at.map(|mm: f64| crate::base::metres_text(mm)));
            at = at.zip(metres(&s.height)).map(|(a, h)| a + h);
        }
        out.push(at.map(crate::base::metres_text));
        out
    }
}

impl BimApp {
    /// Opens Create Project.
    pub(crate) fn open_create_project(&mut self) {
        self.create_project = Some(CreateProject::new(Words::of(self.lang.project())));
    }

    /// Makes the project Create asked for and saves it where it said; the start page goes.
    pub(crate) fn create(&mut self, c: Creating) {
        if self.run("new", c.params).is_none() {
            return;
        }
        self.retire_autosave();
        self.reopened();
        if !self.save_to(&c.path) {
            return;
        }
        // The architect's model, linked once the project has a folder to find it from, and saved with it: in the 3D view, out
        // of the plans until they are flattened.
        if let Some(link) = c.link
            && (self.run("link", json!({"path": link, "in_plans": false})).is_none() || !self.save_to(&c.path))
        {
            return;
        }
        self.status = crate::Status { text: tf("Project created in {}", &[&c.path.display().to_string()]), error: false };
        self.autosave_document();
    }

    /// The architect's model chosen in the form: kept to link, once it is read (`levels_in`), or why it couldn't be.
    pub(crate) fn follow_architecture(&mut self, file: std::path::PathBuf) {
        let read = self.session.execute("levels_in", &json!({"path": file.to_string_lossy()}));
        let Some(f) = self.create_project.as_mut() else { return };
        match read {
            Ok(_) => f.follow(&file),
            Err(e) => f.error = Some(e),
        }
    }

    pub(crate) fn create_project_window(&mut self, ctx: &Context) {
        let Some(mut f) = self.create_project.take() else { return };
        let level = self.lang.project().word("Level").to_string();
        let elevations = f.elevations();
        let (mut close, mut create, mut browse, mut base) = (false, false, false, false);
        let r = Modal::new(UiId::new("create-project"), t("Create Project")).width(560.0).show(ctx, |ui| {
            crate::popup::body(ui, |ui| {
                ui.add_space(4.0);
                ui.label(
                    RichText::new(t("Its name, the folder it is saved in and its storeys, from the ground up: yours, whatever the architect's model has."))
                        .color(pal().muted),
                );
                ui.add_space(8.0);
                egui::Grid::new("create-project-grid").num_columns(2).spacing([10.0, 8.0]).show(ui, |ui| {
                    ui.label(t("Name:"));
                    ui.add_sized([360.0, 22.0], egui::TextEdit::singleline(&mut f.name).hint_text(t("Project")));
                    ui.end_row();
                    ui.label(t("Folder:"));
                    ui.horizontal(|ui| {
                        let shown = f.folder.as_ref().map_or_else(|| t("none chosen").to_string(), |p| p.display().to_string());
                        ui.label(RichText::new(shown).color(pal().muted));
                        browse = ui.button(t("Choose Folder…")).clicked();
                    });
                    ui.end_row();
                    ui.label(t("Architecture (IFC):"));
                    ui.horizontal(|ui| {
                        let shown = f
                            .architecture
                            .as_ref()
                            .and_then(|p| p.file_name())
                            .map_or_else(|| t("none: link it later").to_string(), |n| n.to_string_lossy().into_owned());
                        ui.label(RichText::new(shown).color(pal().muted));
                        base = ui.button(t("Choose IFC…")).clicked();
                    });
                    ui.end_row();
                    ui.label(t("Storeys:"));
                    let mut count = f.storeys.len();
                    if ui.add(DragValue::new(&mut count).range(1..=MAX_STOREYS)).changed() {
                        f.resize(count, &level);
                    }
                    ui.end_row();
                });
                ui.add_space(8.0);
                ScrollArea::vertical().max_height(260.0).auto_shrink([false, true]).show(ui, |ui| {
                    egui::Grid::new("create-project-storeys").num_columns(3).spacing([10.0, 6.0]).show(ui, |ui| {
                        ui.label(RichText::new(t("Level")).strong());
                        ui.label(RichText::new(t("Height (m)")).strong());
                        ui.label(RichText::new(t("Elevation (m)")).strong());
                        ui.end_row();
                        // Top first, as the levels stand, each with the elevation the heights under it put it at.
                        let at = |i: usize| elevations.get(i).cloned().flatten().unwrap_or_else(|| "?".into());
                        ui.add_sized([240.0, 22.0], egui::TextEdit::singleline(&mut f.top));
                        ui.label(RichText::new(t("over the last storey")).color(pal().muted));
                        ui.label(RichText::new(at(f.storeys.len())).color(pal().muted));
                        ui.end_row();
                        for (i, s) in f.storeys.iter_mut().enumerate().rev() {
                            ui.add_sized([240.0, 22.0], egui::TextEdit::singleline(&mut s.name));
                            ui.add_sized([90.0, 22.0], egui::TextEdit::singleline(&mut s.height));
                            ui.label(RichText::new(at(i)).color(pal().muted));
                            ui.end_row();
                        }
                    });
                });
                if let Some(e) = &f.error {
                    ui.add_space(6.0);
                    ui.label(RichText::new(e).color(pal().error));
                }
            });
            crate::popup::footer(ui, |ui| {
                ui.horizontal(|ui| {
                    create = ui.button(t("Create")).clicked();
                    close = ui.button(t("Cancel")).clicked();
                });
            });
        });
        if browse {
            let dialog = rfd::AsyncFileDialog::new();
            self.ask_folder(ctx, dialog, |app, folder| {
                if let Some(f) = app.create_project.as_mut() {
                    f.folder = Some(folder);
                    f.error = None;
                }
            });
        }
        if base {
            let dialog = rfd::AsyncFileDialog::new().add_filter("IFC", &["ifc"]);
            self.ask_file(ctx, dialog, false, |app, file| app.follow_architecture(file));
        }
        if create {
            match f.creating(self.lang.project().code()) {
                Ok(c) => {
                    self.guard(Pending::Create(Box::new(c)), ctx);
                    return;
                }
                Err(e) => f.error = Some(e),
            }
        }
        if !(close || r.should_close()) {
            self.create_project = Some(f);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_height_is_read_in_metres_with_a_comma_or_a_point() {
        assert_eq!(metres("3"), Some(3000.0));
        assert_eq!(metres(" 2,88 "), Some(2880.0));
        assert_eq!(metres("3.06"), Some(3060.0));
        assert_eq!(metres("0"), None);
        assert_eq!(metres("3m"), None);
        assert_eq!(metres("1e9"), None);
    }

    use bimcraft_core::language::Language;

    #[test]
    fn a_project_starts_as_a_house_of_one_storey_with_its_roof_level_over_it() {
        let f = CreateProject::new(Words::of(Language::Portuguese));
        let storeys: Vec<(&str, &str)> = f.storeys.iter().map(|s| (s.name.as_str(), s.height.as_str())).collect();
        assert_eq!((storeys.as_slice(), f.top.as_str()), ([("Térreo", "2,80")].as_slice(), "Cobertura"));
        let f = CreateProject::new(Words::of(Language::English));
        assert_eq!((f.storeys[0].name.as_str(), f.top.as_str()), ("Ground Floor", "Roof Level"));
        assert_eq!(f.elevations(), [Some("0".to_string()), Some("2,8".to_string())]);
    }

    #[test]
    fn more_storeys_follow_the_last_and_the_top_level_keeps_its_name() {
        let w = Words { ground: "Térreo", roof: "Cobertura" };
        let mut f = CreateProject::new(w);
        f.resize(3, "Nível");
        let names: Vec<&str> = f.storeys.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, ["Térreo", "Nível 2", "Nível 3"]);
        assert!(f.storeys.iter().all(|s| s.height == "2,80"));
        assert_eq!(f.top, "Cobertura");
        assert_eq!(f.elevations().last(), Some(&Some("8,4".to_string())));
        f.storeys[1].height = "x".into();
        assert_eq!(f.elevations()[2..], [None, None], "past a height that isn't one, no elevation");
        f.resize(2, "Nível");
        assert_eq!(f.storeys.len(), 2);
    }

    #[test]
    fn the_architect_s_model_leaves_the_storeys_as_they_are() {
        let mut f = CreateProject::new(Words { ground: "Térreo", roof: "Cobertura" });
        f.follow(std::path::Path::new("/projetos/Arquitetura.ifc"));
        assert_eq!(f.name, "Arquitetura");
        assert_eq!((f.storeys.len(), f.top.as_str()), (1, "Cobertura"));
    }

    #[test]
    fn what_is_missing_is_said_before_anything_is_made() {
        let mut f = CreateProject::new(Words { ground: "Level 1", roof: "Level 2" });
        assert!(f.creating("pt").is_err(), "no name");
        f.name = "Casa".into();
        assert!(f.creating("pt").is_err(), "no folder");
        let dir = tempfile::tempdir().unwrap();
        f.folder = Some(dir.path().to_path_buf());
        f.storeys[0].height = "x".into();
        assert!(f.creating("pt").is_err(), "a height that isn't one");
        f.storeys[0].height = "3,06".into();
        f.top = "Level 1".into();
        assert!(f.creating("pt").is_err(), "two levels of one name");
        f.top = "Cobertura".into();
        let c = f.creating("pt").unwrap();
        assert_eq!(c.path, dir.path().join("Casa.bimcraft"));
        assert_eq!(c.params, json!({"name": "Casa", "language": "pt", "heights": [3060.0], "names": ["Level 1", "Cobertura"]}));
        std::fs::write(&c.path, "x").unwrap();
        assert!(f.creating("pt").is_err(), "a file already there is never overwritten");
    }
}
