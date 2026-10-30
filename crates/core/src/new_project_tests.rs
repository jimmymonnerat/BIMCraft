//! A new project with its storeys: `new` with `heights` (each storey's, from the ground up) and `names` (each level's), as the
//! desktop's Create Project asks for them.

use serde_json::json;

use crate::engine::Session;

/// The project's levels, lowest first: (name, elevation).
fn levels(s: &Session) -> Vec<(String, f64)> {
    let mut l: Vec<(String, f64)> = s.model.levels().into_iter().map(|(_, l)| (l.name.clone(), l.elevation)).collect();
    l.sort_by(|a, b| a.1.total_cmp(&b.1));
    l
}

#[test]
fn a_new_project_without_storeys_has_its_two_levels_as_always() {
    let mut s = Session::new();
    s.execute("new", &json!({"language": "pt"})).unwrap();
    assert_eq!(levels(&s), vec![("Nível 1".to_string(), 0.0), ("Nível 2".to_string(), 3000.0)]);
}

#[test]
fn each_storey_s_height_puts_the_next_level_over_it_and_one_more_level_closes_the_last() {
    let mut s = Session::new();
    let r = s.execute("new", &json!({"name": "Casa", "language": "pt", "heights": [3060, 2880]})).unwrap();
    assert_eq!(levels(&s), vec![("Nível 1".into(), 0.0), ("Nível 2".into(), 3060.0), ("Nível 3".into(), 5940.0)]);
    assert_eq!(r["levels"], json!(3));
    assert_eq!(s.model.name, "Casa");
}

#[test]
fn levels_take_the_names_given() {
    let mut s = Session::new();
    s.execute("new", &json!({"heights": [3000, 3000], "names": ["Térreo", "Superior", "Cobertura"]})).unwrap();
    assert_eq!(levels(&s), vec![("Térreo".into(), 0.0), ("Superior".into(), 3000.0), ("Cobertura".into(), 6000.0)]);
    assert_eq!(s.model.check(), Ok(()));
}

#[test]
fn storeys_that_make_no_sense_are_refused_and_the_project_stays() {
    let mut s = Session::new();
    s.execute("level", &json!({"elevation": 9000})).unwrap();
    for p in [
        json!({"heights": []}),
        json!({"heights": [0]}),
        json!({"heights": [-3000]}),
        json!({"heights": ["3m"]}),
        json!({"heights": [3000], "names": ["A"]}),
        json!({"heights": [3000], "names": ["A", "A"]}),
        json!({"heights": [3000], "names": ["A", ""]}),
        json!({"heights": vec![3000; 201]}),
        json!({"heights": [1e300]}),
        json!({"names": ["A", "B"]}),
        json!({"heigths": [3000]}),
    ] {
        let before = s.model.clone();
        assert!(s.execute("new", &p).is_err(), "{p}");
        assert_eq!(*s.model, *before, "{p}");
    }
}

#[test]
fn the_levels_of_an_architect_s_ifc_are_read_without_opening_it() {
    let dir = std::env::temp_dir().join(format!("bimcraft-test-{}-levels-in", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let ifc = dir.join("arq.ifc");
    let mut arq = Session::new();
    arq.execute("new", &json!({"language": "pt", "heights": [3060, 2880], "names": ["Térreo", "Superior", "Cobertura"]})).unwrap();
    arq.execute("wall", &json!({"points": [[0, 0], [5000, 0]]})).unwrap();
    arq.execute("save", &json!({"path": ifc})).unwrap();
    let mut s = Session::new();
    let before = s.model.clone();
    let r = s.execute("levels_in", &json!({"path": ifc})).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(
        r["levels"],
        json!([
            {"name": "Térreo", "elevation": 0.0},
            {"name": "Superior", "elevation": 3060.0},
            {"name": "Cobertura", "elevation": 5940.0}
        ])
    );
    assert_eq!(*s.model, *before, "nothing is opened or linked");
    assert!(s.execute("levels_in", &json!({"path": "/nonexistent/arq.ifc"})).is_err());
    assert!(s.execute("levels_in", &json!({"path": 5})).is_err());
}
