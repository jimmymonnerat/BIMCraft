use super::*;
use std::time::{Duration, Instant};

/// The app on its start page, its recent files kept in `dir`.
fn home(dir: &std::path::Path, files: &[&std::path::Path]) -> Harness<'static, BimApp> {
    let mut h = app("");
    h.state_mut().keep_recent_in(dir.join("recent.json"));
    for f in files.iter().rev() {
        h.state_mut().note_recent(f);
    }
    h.state_mut().show_start_page();
    h.run();
    h
}

/// Frames until every tile's picture is drawn, or found not to be.
fn settle(h: &mut Harness<BimApp>) {
    let deadline = Instant::now() + Duration::from_secs(30);
    while !h.state().home.settled() {
        assert!(Instant::now() < deadline, "the pictures were not drawn");
        h.step();
        std::thread::sleep(Duration::from_millis(5));
    }
    h.run();
}

fn project(dir: &std::path::Path, name: &str) -> std::path::PathBuf {
    let path = dir.join(name);
    let mut s = Session::new();
    s.execute("wall", &json!({"points": [[0, 0], [6000, 0], [6000, 4000], [0, 4000]], "closed": true})).unwrap();
    s.execute("save", &json!({"path": path})).unwrap();
    path
}

#[test]
fn the_start_page_offers_new_and_open_and_says_where_the_source_is() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = home(dir.path(), &[]);
    assert!(h.query_by_label("Architecture").is_none(), "no ribbon behind it");
    h.get_by_label("Open…");
    h.get_by_label("No recent projects yet");
    h.get_by_label("Source code on GitHub");
    h.get_by_label("This copy doesn't look for updates.");
    assert!(h.query_by_label("Back to Project").is_none(), "an empty project is nothing to go back to");
    press(&mut h, "New Project");
    assert!(!h.state().home.shown);
    h.get_by_label("Architecture");
    // Home brings it back over the project, which Back leaves as it was.
    tool(&mut h, "Wall");
    press(&mut h, "Home");
    assert!(h.state().home.shown);
    h.state_mut().run("wall", json!({"points": [[0, 0], [3000, 0]]}));
    h.run();
    let back = format!("Back to {}", h.state().session.model.name);
    press(&mut h, &back);
    assert!(!h.state().home.shown);
    assert_eq!(walls(&h).len(), 1);
}

#[test]
fn ctrl_d_and_the_back_arrow_go_between_the_start_page_and_the_project() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = home(dir.path(), &[]);
    // Opened on no project, there is nothing behind it to go back to.
    h.key_press_modifiers(Modifiers::COMMAND, Key::D);
    h.run();
    assert!(h.state().home.shown);
    assert!(h.query_by_label("Back").is_none());
    press(&mut h, "New Project");
    h.state_mut().run("wall", json!({"points": [[0, 0], [3000, 0]]}));
    h.key_press_modifiers(Modifiers::COMMAND, Key::D);
    h.run();
    assert!(h.state().home.shown, "Ctrl+D brings it up over the project");
    press(&mut h, "Back");
    assert!(!h.state().home.shown, "the arrow at the top goes back");
    press(&mut h, "Home");
    h.key_press_modifiers(Modifiers::COMMAND, Key::D);
    h.run();
    assert!(!h.state().home.shown, "and so does Ctrl+D");
    assert_eq!(walls(&h).len(), 1);
    // In a field the keys are the field's: Ctrl+D there doesn't leave the project.
    h.get_by_label("View Name").focus();
    h.run();
    h.key_press_modifiers(Modifiers::COMMAND, Key::D);
    h.run();
    assert!(!h.state().home.shown, "typing, Ctrl+D stays in the field");
}

#[test]
fn recent_files_are_tiles_with_a_picture_that_open_with_a_click() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = (project(dir.path(), "office.bimcraft"), project(dir.path(), "school.bimcraft"));
    let gone = dir.path().join("gone.bimcraft");
    let mut h = home(dir.path(), &[&a, &b, &gone]);
    settle(&mut h);
    assert!(h.state().home.has_picture(&std::path::absolute(&a).unwrap()));
    assert!(!h.state().home.has_picture(&std::path::absolute(&gone).unwrap()));
    // A file that's gone can be taken off the list.
    h.get_by_label("gone.bimcraft").click_secondary();
    h.run();
    press(&mut h, "Remove from List");
    assert!(h.query_by_label("gone.bimcraft").is_none());
    assert_eq!(h.state().recent.paths().len(), 2);
    press(&mut h, "school.bimcraft");
    assert!(!h.state().home.shown);
    assert_eq!(h.state().session.path.as_deref().and_then(|p| p.file_name()), b.file_name());
    assert_eq!(walls(&h).len(), 4);
}

#[test]
fn the_footer_says_whether_an_update_is_out() {
    use bimcraft_update::testing::{Fake, release_json};
    use bimcraft_update::{LATEST_URL, Updater, Version};
    let dir = tempfile::tempdir().unwrap();
    let offer = |tag: &str| {
        let net = std::sync::Arc::new(Fake::default().answer(LATEST_URL, release_json(tag, None)));
        Updater::new(net, Version::new(0, 4, 0), None, || {}).inline()
    };
    let mut h = home(dir.path(), &[]);
    h.state_mut().updates = Some(offer("v0.5.0"));
    h.run();
    h.get_by_label("Check for updates").click();
    h.run();
    h.get_by_label("BIMCraft 0.5.0 is available");
    h.get_by_label("Download");
    h.state_mut().updates = Some(offer("v0.4.0"));
    h.state_mut().updates.as_mut().unwrap().check_now();
    h.run();
    h.get_by_label("BIMCraft 0.4.0 is up to date");
}

#[test]
fn create_project_makes_the_storeys_asked_for_and_saves_the_project_in_its_folder() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = home(dir.path(), &[]);
    press(&mut h, "Create Project…");
    assert!(h.state().create_project.is_some(), "the form is open over the start page");
    assert!(h.state().home.shown);
    // Create with nothing filled in says what is missing, inside the form, and makes nothing.
    press(&mut h, "Create");
    assert!(h.state().create_project.as_ref().is_some_and(|f| f.error.is_some()));
    assert!(h.state().home.shown);
    {
        let f = h.state_mut().create_project.as_mut().unwrap();
        f.name = "Casa Maciel".into();
        f.folder = Some(dir.path().to_path_buf());
        f.resize(2, "Level");
        f.storeys[0].name = "Térreo".into();
        f.storeys[0].height = "3,06".into();
        f.storeys[1].name = "Superior".into();
        f.storeys[1].height = "2,88".into();
        f.top = "Cobertura".into();
    }
    h.run();
    press(&mut h, "Create");
    assert!(h.state().create_project.is_none());
    assert!(!h.state().home.shown, "the project opens");
    let m = &h.state().session.model;
    assert_eq!(m.name, "Casa Maciel");
    let mut levels: Vec<(String, f64)> = m.levels().into_iter().map(|(_, l)| (l.name.clone(), l.elevation)).collect();
    levels.sort_by(|a, b| a.1.total_cmp(&b.1));
    assert_eq!(levels, vec![("Térreo".into(), 0.0), ("Superior".into(), 3060.0), ("Cobertura".into(), 5940.0)]);
    assert!(dir.path().join("Casa Maciel.bimcraft").exists(), "saved in the folder chosen");
    assert!(!h.state().dirty());
}

#[test]
fn create_project_s_folder_comes_from_the_system_s_dialog() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = home(dir.path(), &[]);
    press(&mut h, "Create Project…");
    press(&mut h, "Choose Folder…");
    let tx = crate::dialog::BEGUN.with(|b| b.borrow_mut().pop()).expect("a dialog was begun");
    tx.send(Some(vec![dir.path().to_path_buf()])).unwrap();
    h.run();
    assert_eq!(h.state().create_project.as_ref().and_then(|f| f.folder.clone()), Some(dir.path().to_path_buf()));
}

#[test]
fn create_project_keeps_its_own_storeys_whatever_the_architect_s_ifc_has_and_links_it_for_the_3d_view() {
    let dir = tempfile::tempdir().unwrap();
    let ifc = dir.path().join("Arquitetura.ifc");
    let mut arq = Session::new();
    arq.execute("new", &json!({"language": "pt", "heights": [3060, 2880], "names": ["Térreo", "Superior", "Cobertura"]})).unwrap();
    arq.execute("wall", &json!({"points": [[0, 0], [5000, 0], [5000, 4000]]})).unwrap();
    arq.execute("save", &json!({"path": ifc})).unwrap();
    let mut h = home(dir.path(), &[]);
    press(&mut h, "Create Project…");
    {
        // A house of one storey: the ground floor 2,80 m high and the roof level over it.
        let f = h.state().create_project.as_ref().unwrap();
        let storeys: Vec<(&str, &str)> = f.storeys.iter().map(|s| (s.name.as_str(), s.height.as_str())).collect();
        assert_eq!((storeys.as_slice(), f.top.as_str()), ([("Ground Floor", "2,80")].as_slice(), "Roof Level"));
    }
    press(&mut h, "Choose IFC…");
    let tx = crate::dialog::BEGUN.with(|b| b.borrow_mut().pop()).expect("a dialog was begun");
    tx.send(Some(vec![ifc.clone()])).unwrap();
    h.run();
    {
        let f = h.state().create_project.as_ref().unwrap();
        assert_eq!(f.error, None);
        assert_eq!(f.name, "Arquitetura", "named after the file while none is typed");
        assert_eq!(f.storeys.len(), 1, "the storeys are the designer's, not the architect's");
    }
    h.state_mut().create_project.as_mut().unwrap().folder = Some(dir.path().to_path_buf());
    h.run();
    press(&mut h, "Create");
    let m = &h.state().session.model;
    let mut levels: Vec<(String, f64)> = m.levels().into_iter().map(|(_, l)| (l.name.clone(), l.elevation)).collect();
    levels.sort_by(|a, b| a.1.total_cmp(&b.1));
    assert_eq!(levels, vec![("Ground Floor".into(), 0.0), ("Roof Level".into(), 2800.0)]);
    let links: Vec<&bimcraft_core::model::Link> = m
        .elements
        .values()
        .filter_map(|e| match e {
            bimcraft_core::model::Element::Link(l) => Some(l),
            _ => None,
        })
        .collect();
    assert_eq!(links.len(), 1, "the architecture is linked");
    assert_eq!(links[0].path, "Arquitetura.ifc", "beside the project, so the folder can move");
    assert!(!links[0].in_plans, "in the 3D view, out of the plans until they are flattened");
    assert!(!m.loaded.is_empty(), "and read");
    assert!(!h.state().dirty(), "saved with its link");
    // Its plans draw nothing of it: the plan of the ground floor is as an empty project's.
    let seen = bimcraft_core::link::seen(m);
    let ink = |m: &bimcraft_core::model::Model| {
        let ground = m.levels().into_iter().find(|(_, l)| l.elevation == 0.0).map(|(id, _)| id).unwrap();
        bimcraft_core::render::plan(m, ground, 400, 300).prims.len()
    };
    let mut empty = Session::new();
    empty.execute("new", &json!({"heights": [2800], "names": ["Ground Floor", "Roof Level"]})).unwrap();
    assert!(seen.elements.len() > m.elements.len(), "the 3D view has the walls");
    assert_eq!(ink(&seen), ink(&empty.model), "the plan hasn't");
}

#[test]
fn a_copy_that_keeps_settings_shows_the_plumbing_tab_alone_until_asked() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = app("");
    h.state_mut().use_settings(dir.path().join("settings.json"));
    h.run();
    h.get_by_label("Plumbing");
    for other in ["Structure", "Electrical", "Lightning Protection", "Data"] {
        assert!(h.query_by_label(other).is_none(), "{other} is hidden");
    }
    h.state_mut().all_disciplines = true;
    h.run();
    h.get_by_label("Electrical");
    // Kept for the next start.
    let ctx = h.ctx.clone();
    h.state_mut().keep_settings(&ctx);
    let mut again = app("");
    again.state_mut().use_settings(dir.path().join("settings.json"));
    assert!(again.state().all_disciplines);
}
