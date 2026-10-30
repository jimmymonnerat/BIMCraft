//! The pipe network (`fittings.rs`) through the commands a front end runs: `pipe_fittings` places elbows and tees where pipes meet,
//! `material_list` counts what the plumbing takes.

use serde_json::{Value, json};

use crate::engine::Session;
use crate::model::Element;

fn run(s: &mut Session, id: &str, p: Value) -> Value {
    s.execute(id, &p).unwrap_or_else(|e| panic!("{id} {p}: {e}"))
}

fn pipe(s: &mut Session, points: Value, ty: &str, system: &str) {
    run(s, "pipe", json!({"points": points, "type": ty, "offset": 400, "system": system}));
}

/// The components placed, as (family, type, x, y, z over the level, angle).
fn fittings(s: &Session) -> Vec<(String, String, f64, f64, f64, f64)> {
    s.model
        .elements
        .values()
        .filter_map(|e| match e {
            Element::Component(c) => Some((c.family.clone(), c.type_name.clone(), c.at.0, c.at.1, c.offset, c.angle)),
            _ => None,
        })
        .collect()
}

#[test]
fn a_turn_of_90_degrees_in_plan_takes_an_elbow_turned_to_its_pipes_at_their_height() {
    for (end, angle) in [([2000, 1500], 0.0), ([2000, -1500], 90.0)] {
        let mut s = Session::new();
        pipe(&mut s, json!([[0, 0], [2000, 0], end]), "PVC 50", "Água fria");
        let r = run(&mut s, "pipe_fittings", json!({}));
        assert_eq!(r["placed"].as_array().map(Vec::len), Some(1), "{r}");
        assert_eq!(r["open_ends"].as_array().map(Vec::len), Some(2), "{r}");
        assert_eq!(r["unfitted"], json!([]), "{r}");
        assert_eq!(fittings(&s), vec![("Elbow 90".to_string(), "50".to_string(), 2000.0, 0.0, 400.0, angle)], "towards {end:?}");
    }
}

#[test]
fn an_elbow_s_legs_lie_along_its_two_pipes_whichever_way_they_turn() {
    // From the node, the pipes leave along (-1, 0) and (0, -1): the elbow, legs along -x and +y, is turned 90°.
    let mut s = Session::new();
    pipe(&mut s, json!([[0, 0], [2000, 0], [2000, -1500]]), "PVC 50", "");
    run(&mut s, "pipe_fittings", json!({}));
    let (.., angle) = fittings(&s)[0].clone();
    let turn = |v: (f64, f64)| {
        let a = angle.to_radians();
        ((v.0 * a.cos() - v.1 * a.sin()).round(), (v.0 * a.sin() + v.1 * a.cos()).round())
    };
    let legs = [turn((-1.0, 0.0)), turn((0.0, 1.0))];
    assert!(legs.contains(&(-1.0, 0.0)) && legs.contains(&(0.0, -1.0)), "{angle}: {legs:?}");
}

#[test]
fn a_pipe_ending_on_another_between_its_ends_takes_a_tee_with_its_branch_along_the_pipe() {
    for (branch, angle) in [([2000, 1000], 0.0), ([2000, -1000], 180.0)] {
        let mut s = Session::new();
        pipe(&mut s, json!([[0, 0], [4000, 0]]), "PVC 110", "Esgoto");
        pipe(&mut s, json!([[2000, 0], branch]), "PVC 110", "Esgoto");
        let r = run(&mut s, "pipe_fittings", json!({}));
        assert_eq!(r["unfitted"], json!([]), "{r}");
        assert_eq!(fittings(&s), vec![("Tee".to_string(), "110".to_string(), 2000.0, 0.0, 400.0, angle)], "{r}");
        assert_eq!(r["open_ends"].as_array().map(Vec::len), Some(3), "{r}");
    }
}

#[test]
fn running_it_again_leaves_a_fitted_node_alone() {
    let mut s = Session::new();
    pipe(&mut s, json!([[0, 0], [2000, 0], [2000, 1500]]), "PVC 50", "");
    run(&mut s, "pipe_fittings", json!({}));
    let r = run(&mut s, "pipe_fittings", json!({}));
    assert_eq!(r["placed"], json!([]), "{r}");
    assert_eq!(r["already_fitted"], json!(1), "{r}");
    assert_eq!(fittings(&s).len(), 1);
}

#[test]
fn a_diameter_the_family_lacks_becomes_a_type_of_it() {
    let mut s = Session::new();
    pipe(&mut s, json!([[0, 0], [2000, 0], [2000, 1500]]), "Copper 22", "Água quente");
    assert!(!s.model.families["Elbow 90"].types.contains_key("22"));
    run(&mut s, "pipe_fittings", json!({}));
    assert_eq!(s.model.families["Elbow 90"].types["22"]["Diameter"], json!(22.0));
    assert_eq!(s.model.check(), Ok(()));
}

#[test]
fn what_no_family_fits_yet_is_listed_with_why_and_nothing_is_guessed() {
    let mut s = Session::new();
    // In line, from DN 50 to DN 25: a reduction.
    pipe(&mut s, json!([[0, 0], [2000, 0]]), "PVC 50", "");
    pipe(&mut s, json!([[2000, 0], [4000, 0]]), "PVC 25", "");
    // A turn of 45°.
    pipe(&mut s, json!([[0, 5000], [2000, 5000], [3000, 6000]]), "PVC 50", "");
    let r = run(&mut s, "pipe_fittings", json!({}));
    let why: Vec<&str> = r["unfitted"].as_array().unwrap().iter().filter_map(|u| u["why"].as_str()).collect();
    assert!(why.iter().any(|w| w.contains("reduction")), "{r}");
    assert!(why.iter().any(|w| w.contains("45°")), "{r}");
    assert!(fittings(&s).is_empty(), "{r}");
}

#[test]
fn pipes_in_line_of_one_diameter_need_nothing_and_a_dry_run_changes_nothing() {
    let mut s = Session::new();
    pipe(&mut s, json!([[0, 0], [2000, 0], [4000, 0]]), "PVC 50", "");
    pipe(&mut s, json!([[0, 3000], [2000, 3000], [2000, 5000]]), "PVC 50", "");
    let before = s.model.clone();
    let r = run(&mut s, "pipe_fittings", json!({"dry_run": true}));
    assert_eq!(r["placed"].as_array().map(Vec::len), Some(1), "{r}");
    assert_eq!(*s.model, *before);
}

#[test]
fn only_the_system_asked_for_is_fitted() {
    let mut s = Session::new();
    pipe(&mut s, json!([[0, 0], [2000, 0], [2000, 1500]]), "PVC 50", "Água fria");
    pipe(&mut s, json!([[0, 5000], [2000, 5000], [2000, 6500]]), "PVC 110", "Esgoto");
    run(&mut s, "pipe_fittings", json!({"system": "Esgoto"}));
    let f = fittings(&s);
    assert_eq!(f.len(), 1);
    assert_eq!((f[0].1.as_str(), f[0].3), ("110", 5000.0));
}

#[test]
fn the_material_list_sums_pipes_in_metres_and_bars_and_counts_the_fittings() {
    let mut s = Session::new();
    pipe(&mut s, json!([[0, 0], [4000, 0], [4000, 3500]]), "PVC 50", "Água fria");
    pipe(&mut s, json!([[0, 5000], [6500, 5000]]), "PVC 110", "Esgoto");
    run(&mut s, "pipe_fittings", json!({}));
    let path = std::env::temp_dir().join(format!("bimcraft-test-{}-materials.csv", std::process::id()));
    let r = run(&mut s, "material_list", json!({"path": path}));
    let csv = std::fs::read_to_string(&path).unwrap();
    let _ = std::fs::remove_file(&path);
    assert_eq!(
        r["pipes"],
        json!([
            {"system": "Esgoto", "type": "PVC 110", "diameter": 110.0, "length_m": 6.5, "bars": 2},
            {"system": "Água fria", "type": "PVC 50", "diameter": 50.0, "length_m": 7.5, "bars": 2}
        ]),
        "{r}"
    );
    assert_eq!(r["components"], json!([{"category": "pipefitting", "family": "Elbow 90", "type": "50", "count": 1}]), "{r}");
    assert!(csv.contains("Água fria;PVC 50;m;7,5;2 x 6 m"), "{csv}");
    assert!(csv.contains(";Elbow 90 50;un;1;"), "{csv}");
}

#[test]
fn a_project_in_portuguese_places_its_own_families() {
    let mut s = Session::new();
    run(&mut s, "new", json!({"language": "pt"}));
    let ty = s.model.types.runs["pipe"].keys().next().cloned().unwrap();
    pipe(&mut s, json!([[0, 0], [2000, 0], [2000, 1500]]), &ty, "Água fria");
    let r = run(&mut s, "pipe_fittings", json!({}));
    assert_eq!(r["placed"][0]["family"], json!("Joelho 90"), "{r}");
}

#[test]
fn hostile_parameters_are_refused_and_change_nothing() {
    let mut s = Session::new();
    for (id, p) in [
        ("pipe_fittings", json!({"sytem": "x"})),
        ("pipe_fittings", json!({"dry_run": "yes"})),
        ("material_list", json!({"bar": -1})),
        ("material_list", json!({"bar": f64::MAX})),
        ("material_list", json!({"path": 3})),
    ] {
        let before = s.model.clone();
        assert!(s.execute(id, &p).is_err(), "{id} {p}");
        assert_eq!(*s.model, *before);
    }
}
