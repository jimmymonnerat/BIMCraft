//! Where pipes meet: the network the pipes of a project make, read from their ends, and the fittings it asks for. A pipe's end
//! is a connector found by its place: ends that meet (within the larger pipe's radius) are one node, and an end that lands on
//! another pipe between its ends is a node of three, a tee. A node of two pipes turning 90° in plan is an elbow, of three with
//! two in line and the third square to them in plan a tee; one end alone is an open end, which the network reports so it can
//! be checked for continuity. The `pipe_fittings` command places the starter families (`fixtures::pipework`) there, of a type
//! of the pipe's diameter, and skips a node that already has a fitting. What it can't fit yet (a reduction, a turn that isn't
//! 90°, a riser's turn, a cross) it lists with why, never guessing.

use crate::geom::P2;
use crate::model::{Element, Id, Model, TOL};
use crate::view3d::{P3, dot, sub, unit};

/// The cosine under which two directions are taken as opposite (in line): within 3°.
const IN_LINE: f64 = -0.9986;
/// How far from square a turn may be and still take a 90° elbow or a tee's branch: 3°.
const SQUARE: f64 = 0.0523;
/// How far up or down a direction may lean and still be taken as level, in plan (a drain's fall: up to about 3°).
const LEVEL: f64 = 0.0523;
/// The most a node may reach to gather ends, in mm: no pipe's ends meet a pipe's width away.
const MAX_REACH: f64 = 100.0;

/// One pipe's end at a node: the pipe, the way it leaves the node (a unit vector), its outside diameter.
#[derive(Clone, Debug, PartialEq)]
pub struct End {
    pub pipe: Id,
    pub dir: P3,
    pub diameter: f64,
}

/// What a node needs.
#[derive(Clone, Debug, PartialEq)]
pub enum Kind {
    /// A pipe's end that meets nothing.
    Open,
    /// Two pipes in line of one diameter: nothing.
    Straight,
    /// Two pipes turning 90° in plan: an elbow whose legs go along -x and +y once turned `angle` degrees.
    Elbow { angle: f64 },
    /// Two in line and one square to them in plan: a tee whose run goes along x and branch along +y once turned `angle`.
    Tee { angle: f64 },
    /// What no starter family fits yet, and why.
    Unfitted(String),
}

/// Where ends meet: the point (mm), the level of the first pipe there, the ends, and what it needs.
#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    pub at: P3,
    pub level: crate::model::Guid,
    pub ends: Vec<End>,
    pub kind: Kind,
}

impl Node {
    /// The diameter its fitting takes: its pipes' largest.
    pub fn diameter(&self) -> f64 {
        self.ends.iter().map(|e| e.diameter).fold(0.0, f64::max)
    }
}

fn plan(d: P3) -> P2 {
    P2(d[0], d[1])
}

fn level(d: P3) -> bool {
    d[2].abs() <= LEVEL
}

/// The angle in degrees, counter-clockwise from east, of a direction in plan.
fn bearing(d: P2) -> f64 {
    let a = d.1.atan2(d.0).to_degrees();
    let a = if a < 0.0 { a + 360.0 } else { a };
    // 360 is 0, and a hair under a whole degree is that degree, so a turned fitting reads as it was meant.
    let r = a.round();
    let a = if (a - r).abs() < 1e-6 { r } else { a };
    if a >= 360.0 { a - 360.0 } else { a }
}

/// What two or three ends ask for.
fn classify(ends: &[End]) -> Kind {
    match ends {
        [_] => Kind::Open,
        [a, b] => {
            let cos = dot(a.dir, b.dir);
            if cos <= IN_LINE {
                if (a.diameter - b.diameter).abs() > TOL {
                    return Kind::Unfitted(format!("a reduction from DN {} to DN {}: no reducer family yet", a.diameter, b.diameter));
                }
                return Kind::Straight;
            }
            if !(level(a.dir) && level(b.dir)) {
                return Kind::Unfitted("a turn up or down (a riser's): fitted by hand for now".into());
            }
            let turn = cos.clamp(-1.0, 1.0).acos().to_degrees();
            if cos.abs() > SQUARE {
                return Kind::Unfitted(format!("a turn of {:.0}°: only 90° elbows so far", 180.0 - turn));
            }
            if (a.diameter - b.diameter).abs() > TOL {
                return Kind::Unfitted(format!("an elbow from DN {} to DN {}: no reducing elbow yet", a.diameter, b.diameter));
            }
            // The legs go along -x and +y: -x on the end whose turn to the other is clockwise.
            let (p, q) = (plan(a.dir).unit(), plan(b.dir).unit());
            let (first, _) = if p.cross(q) < 0.0 { (p, q) } else { (q, p) };
            Kind::Elbow { angle: bearing(first * -1.0) }
        }
        [_, _, _] => {
            if !ends.iter().all(|e| level(e.dir)) {
                return Kind::Unfitted("a tee with a branch up or down: fitted by hand for now".into());
            }
            for i in 0..3 {
                let (x, y) = ((i + 1) % 3, (i + 2) % 3);
                let (run_a, run_b, branch) = (&ends[x], &ends[y], &ends[i]);
                if dot(run_a.dir, run_b.dir) > IN_LINE {
                    continue;
                }
                if dot(run_a.dir, branch.dir).abs() > SQUARE {
                    return Kind::Unfitted("a branch that isn't square to its run: only 90° tees so far".into());
                }
                if ends.iter().any(|e| (e.diameter - branch.diameter).abs() > TOL) {
                    return Kind::Unfitted("a tee of more than one diameter: no reducing tee yet".into());
                }
                return Kind::Tee { angle: bearing(plan(branch.dir).unit().perp() * -1.0) };
            }
            Kind::Unfitted("three pipes with no two in line".into())
        }
        _ => Kind::Unfitted(format!("{} pipes meet: no cross family yet", ends.len())),
    }
}

/// The pipes of `m` (of `system`, when given) as nodes where their ends are.
pub fn network(m: &Model, system: Option<&str>) -> Vec<Node> {
    // Each pipe: its id, level, ends and diameter.
    let pipes: Vec<(Id, crate::model::Guid, P3, P3, f64)> = m
        .elements
        .iter()
        .filter(|(_, e)| e.category() == "pipe")
        .filter_map(|(id, e)| {
            let r = e.run()?;
            if system.is_some_and(|s| r.system != s) {
                return None;
            }
            let g = m.run_geom(e)?;
            (g.length() > TOL).then_some((*id, r.level, g.a, g.b, g.width))
        })
        .collect();
    let mut nodes: Vec<Node> = Vec::new();
    for (id, lvl, a, b, d) in &pipes {
        for (at, other) in [(*a, *b), (*b, *a)] {
            let end = End { pipe: *id, dir: unit(sub(other, at)), diameter: *d };
            let reach = |n: &Node| (n.diameter().max(*d) / 2.0).clamp(TOL, MAX_REACH);
            match nodes.iter_mut().find(|n| dist(n.at, at) <= reach(n)) {
                Some(n) => n.ends.push(end),
                None => nodes.push(Node { at, level: *lvl, ends: vec![end], kind: Kind::Open }),
            }
        }
    }
    // An end alone on another pipe between its ends: a tee, the pipe it lands on running both ways.
    for n in nodes.iter_mut().filter(|n| n.ends.len() == 1) {
        let mine = n.ends[0].pipe;
        let hit = pipes.iter().find(|(id, _, a, b, d)| {
            *id != mine && {
                let (t, off) = along(n.at, *a, *b);
                let len = dist(*a, *b);
                t > d / 2.0 && t < len - d / 2.0 && off <= (d.max(n.ends[0].diameter) / 2.0).clamp(TOL, MAX_REACH)
            }
        });
        if let Some((id, _, a, b, d)) = hit {
            n.ends.push(End { pipe: *id, dir: unit(sub(*b, *a)), diameter: *d });
            n.ends.push(End { pipe: *id, dir: unit(sub(*a, *b)), diameter: *d });
        }
    }
    for n in &mut nodes {
        n.kind = classify(&n.ends);
    }
    nodes
}

fn dist(a: P3, b: P3) -> f64 {
    let d = sub(a, b);
    dot(d, d).sqrt()
}

/// How far along `a`→`b` the point `p` is, and how far off the line.
fn along(p: P3, a: P3, b: P3) -> (f64, f64) {
    let u = unit(sub(b, a));
    let t = dot(sub(p, a), u);
    let foot = [a[0] + u[0] * t, a[1] + u[1] * t, a[2] + u[2] * t];
    (t, dist(p, foot))
}

/// A diameter as a type is named: "50", "32.5".
pub fn type_name(diameter: f64) -> String {
    let r = (diameter * 10.0).round() / 10.0;
    if r.fract() == 0.0 { format!("{r:.0}") } else { format!("{r}") }
}

/// Whether a pipe fitting of `m` already stands at `at` (within `reach` mm): a node fitted before, or by hand.
pub fn fitted(m: &Model, at: P3, reach: f64) -> bool {
    m.elements.values().any(|e| match e {
        Element::Component(c) => {
            m.families.get(&c.family).is_some_and(|f| f.category == crate::family::Category::PipeFitting) && {
                let z = m.elevation(c.level) + c.offset;
                dist([c.at.0, c.at.1, z], at) <= reach
            }
        }
        _ => false,
    })
}
