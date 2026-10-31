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

/// An architect's model of two storeys saved as IFC: a room on the ground floor, one wall upstairs.
fn architect(test: &str) -> std::path::PathBuf {
    let ifc = folder(test).join("Arquitetura.ifc");
    let mut arq = Session::new();
    run(&mut arq, "new", json!({"language": "pt", "heights": [3000, 3000], "names": ["Térreo", "Superior", "Cobertura"]}));
    run(&mut arq, "wall", json!({"points": [[0, 0], [6000, 0], [6000, 4000], [0, 4000]], "closed": true}));
    run(&mut arq, "wall", json!({"points": [[0, 0], [6000, 0]], "level": "Superior"}));
    run(&mut arq, "save", json!({"path": ifc}));
    ifc
}

/// The CAD drawings placed in the project: (level's name, drawing's name).
fn drawings(s: &Session) -> Vec<(String, String)> {
    let mut d: Vec<(String, String)> = s
        .model
        .elements
        .values()
        .filter_map(|e| match e {
            Element::CadImport(ci) => Some((s.model.level(ci.level).map(|l| l.name.clone()).unwrap_or_default(), ci.cad.clone())),
            _ => None,
        })
        .collect();
    d.sort();
    d
}

#[test]
fn flattening_the_architect_s_model_makes_a_drawing_of_lines_for_each_storey_and_takes_the_link_away() {
    let ifc = architect("flatten");
    let mut s = Session::new();
    run(&mut s, "new", json!({"language": "pt", "heights": [3000, 3000], "names": ["Térreo", "Superior", "Cobertura"]}));
    run(&mut s, "link", json!({"path": ifc}));
    let r = run(&mut s, "flatten_link", json!({}));
    // The roof's storey too: it sees the walls under it, down to 15 % of its height under its level.
    let storey = |l: &str| (l.to_string(), format!("Arquitetura - {l}"));
    assert_eq!(drawings(&s), vec![storey("Cobertura"), storey("Superior"), storey("Térreo")], "{r}");
    assert!(!s.model.elements.values().any(|e| matches!(e, Element::Link(_))), "the link goes: its lines stand for it");
    assert_eq!(s.model.check(), Ok(()));
    // The ground floor's lines are the walls' faces where the plan cuts them, in millimetres where the walls are.
    let cad = &s.model.cads["Arquitetura - Térreo"];
    let lines: Vec<(P2, P2)> = cad
        .dxf()
        .entities
        .iter()
        .flat_map(|e| crate::cad::paths(&e.shape))
        .flat_map(|(pts, closed)| {
            let close = closed.then(|| pts.last().copied().zip(pts.first().copied())).flatten();
            pts.windows(2).map(|w| (w[0], w[1])).chain(close).collect::<Vec<_>>()
        })
        .collect();
    assert!(lines.len() >= 8, "{} lines", lines.len());
    let along_south = lines.iter().filter(|(a, b)| (a.1 - b.1).abs() < 1.0 && (a.0 - b.0).abs() > 5000.0).count();
    assert!(along_south >= 2, "both faces of the south wall: {lines:?}");
    assert!(lines.iter().all(|(a, b)| a.0 > -500.0 && b.0 < 6500.0 && a.1 > -500.0 && b.1 < 4500.0), "where the building is");
    // One undo step brings the link back and takes the drawings away.
    run(&mut s, "undo", json!({}));
    assert!(drawings(&s).is_empty());
    assert!(s.model.elements.values().any(|e| matches!(e, Element::Link(_))));
}

#[test]
fn flattening_can_keep_the_link_and_take_only_some_storeys() {
    let ifc = architect("flatten-some");
    let mut s = Session::new();
    run(&mut s, "new", json!({"language": "pt", "heights": [3000, 3000], "names": ["Térreo", "Superior", "Cobertura"]}));
    run(&mut s, "link", json!({"path": ifc}));
    run(&mut s, "flatten_link", json!({"levels": ["Térreo"], "keep_link": true}));
    assert_eq!(drawings(&s), vec![("Térreo".to_string(), "Arquitetura - Térreo".to_string())]);
    assert!(s.model.elements.values().any(|e| matches!(e, Element::Link(_))));
    // Again: the storey's drawing is made anew, not a second one.
    run(&mut s, "flatten_link", json!({"levels": ["Térreo"], "keep_link": true}));
    assert_eq!(drawings(&s).len(), 1);
    let mut empty = Session::new();
    let e = fails(&mut empty, "flatten_link", json!({}));
    assert!(e.contains("link"), "{e}");
    fails(&mut s, "flatten_link", json!({"levels": ["Nowhere"]}));
    fails(&mut s, "flatten_link", json!({"keep": true}));
}

#[test]
fn a_flattened_storey_keeps_the_outlines_and_leaves_the_hatching_out() {
    // The sample house: its outer walls are hatched where the plan cuts them.
    let ifc = folder("hatch").join("Casa.ifc");
    let mut arq = Session::new();
    arq.script(crate::sample::SAMPLE).unwrap();
    run(&mut arq, "save", json!({"path": ifc}));
    let mut s = Session::new();
    run(&mut s, "link", json!({"path": ifc}));
    let r = run(&mut s, "flatten_link", json!({"levels": ["Level 1"]}));
    let lines = r["drawings"][0]["lines"].as_u64().unwrap();
    assert!((20..2000).contains(&lines), "outlines, not hatching: {lines} lines");
}

#[test]
fn an_ifc2x3_flow_terminal_comes_in_as_a_plumbing_fixture_and_is_flattened() {
    // IFC2x3 (Revit's and ArchiCAD's usual export) writes a WC as an IfcFlowTerminal, with no predefined type.
    let dir = folder("flow-terminal");
    let ifc4 = dir.join("four.ifc");
    let mut arq = Session::new();
    run(&mut arq, "new", json!({"language": "pt"}));
    run(&mut arq, "component", json!({"family": "Vaso sanitário", "at": [1000, 1000]}));
    run(&mut arq, "save", json!({"path": ifc4}));
    let text = std::fs::read_to_string(&ifc4).unwrap();
    let line = text.lines().find(|l| l.contains("=IFCSANITARYTERMINAL(")).unwrap().to_string();
    let old = line.replacen("=IFCSANITARYTERMINAL(", "=IFCFLOWTERMINAL(", 1).replace(",.NOTDEFINED.);", ");");
    let ifc = dir.join("Arquitetura.ifc");
    std::fs::write(&ifc, text.replace(&line, &old)).unwrap();
    let mut s = Session::new();
    run(&mut s, "new", json!({"language": "pt"}));
    run(&mut s, "link", json!({"path": ifc}));
    let r = run(&mut s, "flatten_link", json!({"levels": ["Nível 1"]}));
    assert!(r["drawings"][0]["lines"].as_u64().unwrap() > 0, "{r}");
    let cad = &s.model.cads["Arquitetura - Nível 1"];
    assert!(cad.dxf().entities.iter().any(|e| e.layer == "plumbing"), "the WC is drawn, on the plumbing fixtures' layer");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_distribution_element_as_revit_writes_a_maker_s_fixture_comes_in_and_is_flattened() {
    let dir = folder("distribution-element");
    let ifc4 = dir.join("four.ifc");
    let mut arq = Session::new();
    run(&mut arq, "new", json!({"language": "pt"}));
    run(&mut arq, "component", json!({"family": "Lavatório", "at": [1000, 1000]}));
    run(&mut arq, "save", json!({"path": ifc4}));
    let text = std::fs::read_to_string(&ifc4).unwrap();
    let line = text.lines().find(|l| l.contains("=IFCSANITARYTERMINAL(")).unwrap().to_string();
    let revit = line.replacen("=IFCSANITARYTERMINAL(", "=IFCDISTRIBUTIONELEMENT(", 1).replace(",.NOTDEFINED.);", ");");
    let ifc = dir.join("Arquitetura.ifc");
    std::fs::write(&ifc, text.replace(&line, &revit)).unwrap();
    let mut s = Session::new();
    run(&mut s, "new", json!({"language": "pt"}));
    run(&mut s, "link", json!({"path": ifc}));
    run(&mut s, "flatten_link", json!({"levels": ["Nível 1"]}));
    assert!(s.model.cads["Arquitetura - Nível 1"].dxf().entities.iter().any(|e| e.layer == "plumbing"), "the basin is drawn");
    let _ = std::fs::remove_dir_all(&dir);
}

/// An architect's model of two storeys with a room on the ground floor and two boxes in it: one sunk 400 mm under the
/// floor (a box in the ground), one hung 2000 mm over it (a cabinet on the wall).
fn boxes(test: &str) -> std::path::PathBuf {
    let ifc = folder(test).join("Arquitetura.ifc");
    let mut arq = Session::new();
    run(&mut arq, "new", json!({"language": "pt", "heights": [3000, 3000], "names": ["Térreo", "Superior", "Cobertura"]}));
    run(&mut arq, "wall", json!({"points": [[0, 0], [6000, 0], [6000, 4000], [0, 4000]], "closed": true}));
    let square = json!([[0, 0], [600, 0], [600, 600], [0, 600]]);
    run(&mut arq, "family", json!({"name": "Caixa", "types": {"Caixa": {}}, "forms": [{"profile": square, "to": 300}]}));
    run(&mut arq, "component", json!({"family": "Caixa", "at": [1000, 1000], "offset": -400}));
    run(&mut arq, "component", json!({"family": "Caixa", "at": [3000, 1000], "offset": 2000}));
    run(&mut arq, "save", json!({"path": ifc}));
    ifc
}

/// A project of the same storeys linking `ifc`.
fn linking(ifc: &std::path::Path) -> Session {
    let mut s = Session::new();
    run(&mut s, "new", json!({"language": "pt", "heights": [3000, 3000], "names": ["Térreo", "Superior", "Cobertura"]}));
    run(&mut s, "link", json!({"path": ifc}));
    s
}

/// Whether drawing `cad` shows a line with a point inside the box from `lo` to `hi`, of what its import on the ground floor
/// shows.
fn drawn_in(s: &Session, cad: &str, lo: (f64, f64), hi: (f64, f64)) -> bool {
    let shows = s.model.elements.values().find_map(|e| match e {
        Element::CadImport(ci) if ci.cad == cad => Some(ci.clone()),
        _ => None,
    });
    let Some(ci) = shows else { return false };
    s.model.cads[cad]
        .dxf()
        .entities
        .iter()
        .filter(|e| s.model.cads[cad].shows(e, &ci))
        .flat_map(|e| crate::cad::paths(&e.shape))
        .flat_map(|(pts, _)| pts)
        .any(|p| p.0 > lo.0 && p.0 < hi.0 && p.1 > lo.1 && p.1 < hi.1)
}

#[test]
fn each_storey_is_cut_where_it_is_asked_and_seen_down_to_where_it_is_asked() {
    let ifc = boxes("cut");
    let mut s = linking(&ifc);
    let (sunk, hung) = (((900.0, 900.0), (1700.0, 1700.0)), ((2900.0, 900.0), (3700.0, 1700.0)));
    // As a storey 3 m high is cut unless it's said: at 60 % of its height, seen down to 15 % of it under its floor.
    let r = run(&mut s, "flatten_link", json!({"levels": ["Térreo"], "keep_link": true}));
    let d = &r["drawings"][0];
    assert_eq!((d["cut"].as_f64(), d["bottom"].as_f64()), (Some(1800.0), Some(-450.0)), "{r}");
    let cad = "Arquitetura - Térreo";
    assert!(drawn_in(&s, cad, sunk.0, sunk.1), "the box in the ground is seen");
    assert!(!drawn_in(&s, cad, hung.0, hung.1), "the cabinet is over the cut");
    // Cut higher and seen down to the floor alone: the other way round.
    run(&mut s, "flatten_link", json!({"levels": [{"level": "Térreo", "cut": 2200, "bottom": 0}], "keep_link": true}));
    assert!(!drawn_in(&s, cad, sunk.0, sunk.1));
    assert!(drawn_in(&s, cad, hung.0, hung.1));
    // One cut for every storey, said once.
    let r = run(&mut s, "flatten_link", json!({"cut": 1500, "bottom": -300, "keep_link": true}));
    let all = r["drawings"].as_array().unwrap();
    assert!(!all.is_empty() && all.iter().all(|d| d["cut"] == 1500.0 && d["bottom"] == -300.0), "{r}");
    // A bottom over the cut, a height that isn't one, or a word it doesn't know, is refused.
    fails(&mut s, "flatten_link", json!({"cut": 1000, "bottom": 1200}));
    fails(&mut s, "flatten_link", json!({"levels": [{"level": "Térreo", "cut": "high"}]}));
    fails(&mut s, "flatten_link", json!({"levels": [{"level": "Térreo", "under": 3}]}));
    let _ = std::fs::remove_dir_all(folder("cut"));
}

#[test]
fn a_flattened_storey_draws_each_element_as_an_object_of_its_own_and_the_link_stays_for_the_3d_view() {
    let ifc = boxes("objects");
    let mut s = linking(&ifc);
    run(&mut s, "flatten_link", json!({"levels": ["Térreo"], "keep_link": true}));
    // The link stays, for the 3D view and sections; plans draw its lines now, not it.
    let link = s.model.elements.values().find_map(|e| match e {
        Element::Link(l) => Some(l.clone()),
        _ => None,
    });
    assert_eq!(link.map(|l| l.in_plans), Some(false));
    let cad = "Arquitetura - Térreo";
    let objects: std::collections::BTreeSet<String> = s.model.cads[cad].dxf().entities.iter().map(|e| e.object.clone()).collect();
    assert_eq!(objects.len(), 5, "four walls and the box in the ground, each its own: {objects:?}");
    let layers: std::collections::BTreeSet<String> = s.model.cads[cad].dxf().entities.iter().map(|e| e.layer.clone()).collect();
    assert_eq!(layers, ["furniture".to_string(), "wall".to_string()].into(), "a layer for what each is");
    // An object erased stays erased when the storey is flattened again, as when the architect's model changes.
    let import = |s: &Session| {
        s.model.elements.iter().find(|(_, e)| matches!(e, Element::CadImport(ci) if ci.cad == cad)).map(|(id, _)| *id).unwrap()
    };
    let id = import(&s);
    let r = run(&mut s, "cad_erase", json!({"id": id, "at": [1300, 1005]}));
    assert_eq!(r["layer"], "furniture", "{r}");
    assert!(!drawn_in(&s, cad, (900.0, 900.0), (1700.0, 1700.0)), "the box is erased");
    run(&mut s, "flatten_link", json!({"levels": ["Térreo"], "keep_link": true}));
    assert!(!drawn_in(&s, cad, (900.0, 900.0), (1700.0, 1700.0)), "and stays erased");
    assert!(drawn_in(&s, cad, (-500.0, -500.0), (6500.0, 500.0)), "the rest is drawn");
    let _ = std::fs::remove_dir_all(folder("objects"));
}

#[test]
fn a_gutter_comes_in_as_it_is_drawn_and_not_as_a_straight_pipe() {
    // Revit writes a gutter as a pipe segment whose type is a gutter: its body, a trough, is no straight run.
    let dir = folder("gutter");
    let ifc4 = dir.join("four.ifc");
    let mut arq = Session::new();
    run(&mut arq, "pipe", json!({"points": [[0, 0], [4000, 0]], "type": "PVC 50"}));
    run(&mut arq, "save", json!({"path": ifc4}));
    let text = std::fs::read_to_string(&ifc4).unwrap();
    assert!(text.contains(".RIGIDSEGMENT."), "a pipe segment");
    let ifc = dir.join("Arquitetura.ifc");
    std::fs::write(&ifc, text.replace(".RIGIDSEGMENT.", ".GUTTER.")).unwrap();
    let mut s = Session::new();
    run(&mut s, "link", json!({"path": ifc}));
    let seen = crate::link::seen(&s.model);
    assert!(!seen.elements.values().any(|e| e.category() == "pipe"), "not a pipe");
    let generic = seen.elements.values().any(|e| match e {
        Element::Component(c) => seen.families.get(&c.family).is_some_and(|f| f.category == crate::family::Category::Generic),
        _ => false,
    });
    assert!(generic, "a component of its own shape");
    let _ = std::fs::remove_dir_all(&dir);
}
