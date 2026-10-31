//! The base a project is drawn over, set as a designer sets it: its scale by two points of a known distance (`scale_by_reference`)
//! and the origin every discipline shares. A drawing (DXF, DWG) is scaled; a linked model carries its units, so its scale is checked.

use serde_json::{Value, json};

use crate::engine::Session;
use crate::geom::P2;
use crate::model::{Element, cad_place};

fn run(s: &mut Session, id: &str, p: Value) -> Value {
    s.execute(id, &p).unwrap_or_else(|e| panic!("{id} {p}: {e}"))
}

fn fails(s: &mut Session, id: &str, p: Value) -> String {
    let before = s.model.clone();
    let e = s.execute(id, &p).expect_err(&format!("{id} {p} should fail"));
    assert_eq!(*s.model, *before, "a refused {id} changes nothing");
    e
}

fn folder(test: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("bimcraft-ref-{}-{test}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A wall 15 units thick and 400 long, drawn in a DXF that says nothing of its units: an architect's plan in centimetres.
fn base(test: &str) -> String {
    let lines = [((0.0, 0.0), (400.0, 0.0)), ((0.0, 15.0), (400.0, 15.0))]
        .iter()
        .map(|((ax, ay), (bx, by))| format!("0\nLINE\n8\n0\n10\n{ax}\n20\n{ay}\n11\n{bx}\n21\n{by}\n"))
        .collect::<String>();
    let text = format!("0\nSECTION\n2\nENTITIES\n{lines}0\nENDSEC\n0\nEOF\n");
    let path = folder(test).join("base.dxf");
    std::fs::write(&path, text).unwrap();
    path.to_string_lossy().into_owned()
}

/// Where the drawing's point `p` (its own units) stands in the project, mm.
fn placed(s: &Session, p: (f64, f64)) -> P2 {
    let ci = s.model.elements.values().find_map(|e| match e {
        Element::CadImport(ci) => Some(ci.clone()),
        _ => None,
    });
    cad_place(&ci.unwrap())(P2(p.0, p.1))
}

fn close(a: P2, b: P2) -> bool {
    a.dist(b) < 1e-6
}

#[test]
fn a_drawing_is_scaled_so_two_points_clicked_are_the_distance_typed_and_the_first_stays_put() {
    let mut s = Session::new();
    run(&mut s, "import_cad", json!({"path": base("scale"), "units": "mm", "position": "origin"}));
    // Read as mm, the wall is 15 mm thick: the two faces clicked are 15 apart, and they are 150 mm.
    let first = placed(&s, (100.0, 0.0));
    let r = run(&mut s, "scale_by_reference", json!({"from": [first.0, first.1], "to": [first.0, first.1 + 15.0], "length": 150}));
    assert_eq!(r["factor"], json!(10.0), "{r}");
    assert!(close(placed(&s, (100.0, 0.0)), first), "the first point clicked stays where it was");
    assert!(close(placed(&s, (100.0, 15.0)), P2(first.0, first.1 + 150.0)), "the second is the distance typed from it");
    assert!(close(placed(&s, (400.0, 0.0)), P2(first.0 + 3000.0, first.1)), "and the rest follows: 400 cm are 4 m");
    // One undo step takes it back.
    run(&mut s, "undo", json!({}));
    assert!(close(placed(&s, (100.0, 15.0)), P2(first.0, first.1 + 15.0)));
}

#[test]
fn a_linked_model_carries_its_units_so_its_scale_is_checked_not_changed() {
    let dir = folder("link");
    let ifc = dir.join("arq.ifc");
    let mut arq = Session::new();
    run(&mut arq, "wall", json!({"points": [[0, 0], [5000, 0]]}));
    run(&mut arq, "save", json!({"path": ifc}));
    let mut s = Session::new();
    run(&mut s, "link", json!({"path": ifc}));
    let r = run(&mut s, "scale_by_reference", json!({"from": [0, 0], "to": [5000, 0], "length": 5000}));
    assert_eq!(r["factor"], json!(1.0), "{r}");
    let e = fails(&mut s, "scale_by_reference", json!({"from": [0, 0], "to": [5000, 0], "length": 50}));
    assert!(e.contains("units"), "{e}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn what_to_scale_and_by_what_must_be_clear() {
    let mut s = Session::new();
    let e = fails(&mut s, "scale_by_reference", json!({"from": [0, 0], "to": [10, 0], "length": 100}));
    assert!(e.contains("nothing"), "no base: {e}");
    let path = base("clear");
    let a = run(&mut s, "import_cad", json!({"path": path, "units": "mm"}))["id"].clone();
    run(&mut s, "import_cad", json!({"cad": "base", "at": [9000, 0]}));
    let e = fails(&mut s, "scale_by_reference", json!({"from": [0, 0], "to": [10, 0], "length": 100}));
    assert!(e.contains("`id`"), "two bases: say which: {e}");
    run(&mut s, "scale_by_reference", json!({"id": a, "from": [0, 0], "to": [10, 0], "length": 100}));
    for p in [
        json!({"id": a, "from": [0, 0], "to": [0, 0], "length": 100}),
        json!({"id": a, "from": [0, 0], "to": [10, 0], "length": 0}),
        json!({"id": a, "from": [0, 0], "to": [10, 0], "length": -5}),
        json!({"id": a, "from": [0, 0], "to": [10, 0]}),
        json!({"id": a, "from": [0, 0], "to": [1e-9, 0], "length": 1e9}),
        json!({"id": 100000, "from": [0, 0], "to": [10, 0], "length": 100}),
        json!({"id": a, "form": [0, 0], "to": [10, 0], "length": 100}),
    ] {
        fails(&mut s, "scale_by_reference", p);
    }
}

#[test]
fn the_origin_clicked_becomes_the_project_s_0_0_and_everything_keeps_its_place_on_the_base() {
    let dir = folder("origin");
    let ifc = dir.join("arq.ifc");
    let mut arq = Session::new();
    run(&mut arq, "wall", json!({"points": [[0, 0], [5000, 0], [5000, 4000]]}));
    run(&mut arq, "save", json!({"path": ifc}));
    let mut s = Session::new();
    let link = run(&mut s, "link", json!({"path": ifc}))["id"].clone();
    run(&mut s, "import_cad", json!({"path": base("origin"), "units": "mm", "position": "origin"}));
    run(&mut s, "pipe", json!({"points": [[1000, 500], [3000, 500]], "type": "PVC 50"}));
    let before = placed(&s, (0.0, 0.0));
    // The corner of the architect's building: where every discipline puts its 0,0.
    let r = run(&mut s, "set_origin", json!({"at": [5000, 4000]}));
    assert_eq!(r["by"], json!([-5000.0, -4000.0]), "{r}");
    let Some(Element::Link(l)) = s.model.elements.get(link.as_u64().unwrap()) else { panic!("the link") };
    assert_eq!(l.at, P2(-5000.0, -4000.0), "the link moves with the rest");
    assert!(close(placed(&s, (0.0, 0.0)), before - P2(5000.0, 4000.0)), "and the drawing");
    let pipe = s.model.elements.values().find_map(|e| e.run().filter(|_| e.category() == "pipe").cloned()).unwrap();
    assert_eq!((pipe.start, pipe.end), (P2(-4000.0, -3500.0), P2(-2000.0, -3500.0)), "and what was drawn over them");
    assert_eq!(s.model.check(), Ok(()));
    // One undo step takes it all back.
    run(&mut s, "undo", json!({}));
    assert!(close(placed(&s, (0.0, 0.0)), before));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn an_origin_must_be_a_point() {
    let mut s = Session::new();
    for p in [json!({}), json!({"at": "corner"}), json!({"at": [1e300, 0]}), json!({"at": [0, 0], "angle": 5})] {
        fails(&mut s, "set_origin", p);
    }
    let r = run(&mut s, "set_origin", json!({"at": [0, 0]}));
    assert_eq!(r["moved"], json!(0), "the origin where it is moves nothing");
}
