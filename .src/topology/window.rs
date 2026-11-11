//! What moved over the last [`SPAN`], stated on a drawn topology (ADR-0052,
//! amendment 2026-11-10): each link's rate, and each drawn thing's
//! activity — what moved through it — both per second. A publisher keeps one
//! [`Window`] for as long as it publishes and hands it every topology it
//! draws, with the snapshot it drew it from. The window keeps its readings
//! of every figure drawn, about one a second ([`STEP`]), and measures each
//! figure from the newest reading at least [`SPAN`] old — or the oldest it
//! holds, while it has published for less.
//!
//! A drawn thing's activity is the most of the Streams, Journeys or Messages
//! counted on a stage (`Scope::stage`) at and beneath its scope that rose per
//! second — its busiest stage, so a Stream received and routed on one node is
//! not counted twice; the cluster's beneath its nodes, where its mood is read
//! too; never what a test counted at the cluster or beneath a node — or what
//! its links carried into it or out of it, the more of the two, where that is
//! more: a Party and a store have nothing counted beneath them, and what passes
//! through a stage is not counted once coming in and again going out. Nothing
//! moved is zero, and a surface draws a thing at zero still.
//!
//! **A round costs one pass, and no search while the drawing keeps its
//! shape.** A reading is every figure's value in the drawing's own order,
//! and where each count of the snapshot adds is worked out once and kept
//! while the drawing and the snapshot's counts are what they were; a
//! drawing of another shape is read anew, carrying what was read of each
//! figure still drawn. As first built, each drawn thing measured the whole
//! snapshot three times and walked every link, so a round grew with the
//! square of the cluster: 101 ms for 1,202 drawn things, 3,000 counts and
//! 1,000 links (the review of 2026-11-11). **A figure that fell** is a
//! publisher that started over, measured from the reading it fell in;
//! **a clock that steps back** — a time sync, a restored virtual machine —
//! finds every reading later than the new one, and the window starts over;
//! and a figure no longer drawn is forgotten.
//!
//! Until 2026-11-10 a link's rate was its rise since the one publication
//! before (`Topology::rate_since`), about a second, so a link that carried a
//! Message every few seconds read zero in most rounds and its movement came
//! and went; and a node's activity was one while its System Process was
//! alive, so every live node pulsed at the fastest rate whatever it carried.
//! Written once, here: the Essentials' roll states it on the cluster it
//! draws, and a running node on itself (the runtime's `Running::publication`,
//! which `xmip-service` publishes every quarter of a second); a surface draws
//! what was stated and computes no rate of its own.

use std::collections::{HashMap, VecDeque};

use crate::counted::Counted;
use crate::scope::Scope;
use crate::snapshot::{Count, Snapshot};
use crate::topology::{NodeKind, Topology, draw};

/// How far back a [`Window`] measures, in nanoseconds: half a minute, so a
/// link that carries a Message every few seconds reads as moving between
/// them, and one that stopped reads still within half a minute.
pub const SPAN: i64 = 30_000_000_000;

/// How far apart a [`Window`] keeps its readings, the newest apart, in
/// nanoseconds: a second, so it holds about thirty however often its
/// publisher publishes — `xmip-service` every quarter of a second, and at
/// once after every order or act.
const STEP: i64 = 1_000_000_000;

/// What moves through a drawn thing: what each stage of the message path
/// counts (`Counted::at`), on a stage (`Scope::stage`), never a gauge a test
/// keeps beneath a node, as the Essentials' daily backlog's depth.
const THROUGH: [Counted; 3] = [Counted::Streams, Counted::Journeys, Counted::Messages];

/// A publisher's readings of what it drew: the shape it last read a drawing
/// as, where the snapshot's counts add, and its readings, oldest first, a
/// [`STEP`] apart but the newest, back to the newest at least [`SPAN`] old.
#[derive(Clone, Debug, Default)]
pub struct Window {
    shape: Shape,
    plan: Plan,
    readings: VecDeque<Reading>,
    /// How many readings it has taken: the number the next one is given.
    taken: u64,
    /// The number of the first reading each figure is measured from at the
    /// earliest: the first that held it, or the one it fell in.
    since: Vec<u64>,
}

/// One reading: its number, when it was observed, and every figure's value
/// in its [`Shape`]'s order.
#[derive(Clone, Debug)]
struct Reading {
    number: u64,
    at: i64,
    values: Vec<u64>,
}

/// A drawing as a window reads it: each link by its id and its ends, and
/// each drawn thing by its id, its kind and its scope, in the drawing's
/// order. Its figures are each link's volume, in that order, then what was
/// counted beneath each thing, a figure for each kind [`THROUGH`] names.
#[derive(Clone, Debug, Default)]
struct Shape {
    links: Vec<[String; 3]>,
    things: Vec<(String, NodeKind, String)>,
    /// Each link's ends, as the places of the things they are drawn at.
    ends: Vec<[Option<usize>; 2]>,
}

/// One figure of a [`Shape`], by what it is of: a link, or a kind counted
/// beneath a thing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Figure<'s> {
    Link(&'s str),
    Thing(&'s str, usize),
}

impl Shape {
    fn of(topology: &Topology) -> Self {
        let places: HashMap<&str, usize> = topology
            .nodes
            .iter()
            .enumerate()
            .map(|(place, node)| (node.id.as_str(), place))
            .collect();
        let at = |end: &str| places.get(end).copied();
        Self {
            links: topology
                .links
                .iter()
                .map(|link| [link.id.clone(), link.from.clone(), link.to.clone()])
                .collect(),
            things: topology
                .nodes
                .iter()
                .map(|node| (node.id.clone(), node.kind, node.scope.clone()))
                .collect(),
            ends: topology
                .links
                .iter()
                .map(|link| [at(&link.from), at(&link.to)])
                .collect(),
        }
    }

    /// Whether `topology` is drawn in this shape.
    fn is(&self, topology: &Topology) -> bool {
        self.links.len() == topology.links.len()
            && self.things.len() == topology.nodes.len()
            && self
                .links
                .iter()
                .zip(&topology.links)
                .all(|([id, from, to], link)| {
                    *id == link.id && *from == link.from && *to == link.to
                })
            && self
                .things
                .iter()
                .zip(&topology.nodes)
                .all(|((id, kind, scope), node)| {
                    *id == node.id && *kind == node.kind && *scope == node.scope
                })
    }

    /// Every figure, in the order a reading holds them.
    fn figures(&self) -> impl Iterator<Item = Figure<'_>> {
        let links = self.links.iter().map(|[id, _, _]| Figure::Link(id));
        let things = self
            .things
            .iter()
            .flat_map(|(id, _, _)| (0..THROUGH.len()).map(move |kind| Figure::Thing(id, kind)));
        links.chain(things)
    }

    /// Where the thing at `place` is measured: at and beneath its scope, the
    /// cluster's beneath its nodes.
    fn measured(&self, place: usize) -> String {
        let (_, kind, scope) = &self.things[place];
        if *kind == NodeKind::Cluster {
            draw::node_rollup(scope)
        } else {
            scope.clone()
        }
    }
}

/// Where each count a snapshot carries of a kind [`THROUGH`] names adds:
/// its scope and its kind, in the snapshot's order, and the places of the
/// things of a [`Shape`] measured at it or above it — the prefix rule
/// `Scope::contains` says, worked out once — none for a count on no stage.
#[derive(Clone, Debug, Default)]
struct Plan {
    counts: Vec<(String, usize)>,
    /// Where each count's places begin among `places`, and where the last
    /// one's end.
    starts: Vec<usize>,
    places: Vec<usize>,
}

/// Each count `snapshot` carries of a kind [`THROUGH`] names, with the
/// kind's place there.
fn through(snapshot: &Snapshot) -> impl Iterator<Item = (&Count, usize)> {
    snapshot.all_counts().filter_map(|count| {
        let kind = THROUGH.iter().position(|kind| *kind == count.counted)?;
        Some((count, kind))
    })
}

impl Plan {
    fn of(shape: &Shape, snapshot: &Snapshot) -> Self {
        let paths: Vec<String> = (0..shape.things.len())
            .map(|place| shape.measured(place))
            .collect();
        let mut measured: HashMap<&str, Vec<usize>> = HashMap::new();
        for (place, path) in paths.iter().enumerate() {
            measured
                .entry(Scope::new(path).path())
                .or_default()
                .push(place);
        }
        let mut plan = Self {
            starts: vec![0],
            ..Self::default()
        };
        for (count, kind) in through(snapshot) {
            // Only a count on a stage adds: never a test's gauge beneath a node.
            if Scope::new(&count.scope).stage().is_some() {
                let path = Scope::new(&count.scope).path();
                // The root, the path at each slash, and the path itself.
                let above = path.match_indices('/').map(|(at, _)| &path[..at]);
                let whole = (!path.is_empty()).then_some(path);
                for at in std::iter::once("").chain(above).chain(whole) {
                    if let Some(places) = measured.get(at) {
                        plan.places.extend(places);
                    }
                }
            }
            plan.starts.push(plan.places.len());
            plan.counts.push((count.scope.clone(), kind));
        }
        plan
    }

    /// Add what `snapshot` counted to the thing figures of `values`, which
    /// begin at `things`; false, having added nothing whole, where the
    /// snapshot's counts are not the ones this was worked out for.
    fn add(&self, snapshot: &Snapshot, values: &mut [u64], things: usize) -> bool {
        let mut counts = through(snapshot);
        for (place, (scope, kind)) in self.counts.iter().enumerate() {
            match counts.next() {
                Some((count, counted)) if count.scope == *scope && counted == *kind => {
                    for thing in &self.places[self.starts[place]..self.starts[place + 1]] {
                        let figure = things + THROUGH.len() * thing + kind;
                        values[figure] = values[figure].saturating_add(count.value);
                    }
                }
                _ => return false,
            }
        }
        counts.next().is_none()
    }
}

/// `rise` over `nanos`, per second; none over no time.
#[allow(clippy::cast_precision_loss)] // a rate is a figure to read, not a count
fn per_second(rise: u64, nanos: i64) -> Option<f64> {
    (nanos > 0).then(|| rise as f64 * 1e9 / nanos as f64)
}

impl Window {
    /// A window that has read nothing yet.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// State on `topology`, drawn from `snapshot`, each link's rate and each
    /// drawn thing's activity over the window, and keep what it read. A
    /// figure read for the first time, or for the first time since it fell,
    /// and one with no time since the reading it is measured from keep what
    /// was drawn: no interval is no rate, never a zero one.
    pub fn state(&mut self, topology: &mut Topology, snapshot: &Snapshot) {
        let at = topology.observed_unix_nanos;
        if self.readings.back().is_some_and(|newest| newest.at > at) {
            // The clock stepped back: every reading is later than this one.
            self.readings.clear();
        }
        if !self.shape.is(topology) {
            self.reshape(Shape::of(topology));
        }
        let values = self.values(topology, snapshot);
        let since = at.saturating_sub(SPAN);
        while self.readings.get(1).is_some_and(|next| next.at <= since) {
            self.readings.pop_front();
        }
        let number = self.taken;
        self.taken += 1;
        let rates: Vec<Option<f64>> = values
            .iter()
            .enumerate()
            .map(|(figure, value)| self.measure(figure, *value, (at, number)))
            .collect();
        let kept = self.readings.len();
        if kept > 1 && self.readings[kept - 1].at - self.readings[kept - 2].at < STEP {
            self.readings.pop_back();
        }
        self.readings.push_back(Reading { number, at, values });

        for (link, rate) in topology.links.iter_mut().zip(&rates) {
            if let Some(rate) = rate {
                link.rate = *rate;
            }
        }
        // What the links carried into and out of each drawn thing.
        let mut carried = vec![(0.0, 0.0); topology.nodes.len()];
        for (link, [from, to]) in topology.links.iter().zip(&self.shape.ends) {
            if let Some(to) = to {
                carried[*to].0 += link.rate;
            }
            if let Some(from) = from {
                carried[*from].1 += link.rate;
            }
        }
        let things = &rates[topology.links.len()..];
        for ((node, moved), (into, out)) in topology
            .nodes
            .iter_mut()
            .zip(things.chunks(THROUGH.len()))
            .zip(carried)
        {
            if let Some(moved) = moved.iter().flatten().copied().reduce(f64::max) {
                node.activity = moved;
            }
            node.activity = node.activity.max(f64::max(into, out));
        }
    }

    /// Read drawings in `shape` from now on: what was read of each figure
    /// still drawn is carried over, and a figure drawn for the first time is
    /// measured from the next reading.
    fn reshape(&mut self, shape: Shape) {
        let was: HashMap<Figure<'_>, usize> = self
            .shape
            .figures()
            .enumerate()
            .map(|(place, figure)| (figure, place))
            .collect();
        let carried: Vec<Option<usize>> = shape
            .figures()
            .map(|figure| was.get(&figure).copied())
            .collect();
        for reading in &mut self.readings {
            reading.values = carried
                .iter()
                .map(|old| old.map_or(0, |old| reading.values[old]))
                .collect();
        }
        self.since = carried
            .iter()
            .map(|old| old.map_or(self.taken, |old| self.since[old]))
            .collect();
        self.plan = Plan::default();
        self.shape = shape;
    }

    /// Every figure of `topology` drawn from `snapshot`, in its shape's
    /// order, what was counted added where it was worked out to add — and
    /// worked out anew where the snapshot's counts are not what they were.
    fn values(&mut self, topology: &Topology, snapshot: &Snapshot) -> Vec<u64> {
        let things = topology.links.len();
        let mut values: Vec<u64> = topology.links.iter().map(|link| link.volume).collect();
        values.resize(things + THROUGH.len() * topology.nodes.len(), 0);
        if !self.plan.add(snapshot, &mut values, things) {
            values[things..].fill(0);
            self.plan = Plan::of(&self.shape, snapshot);
            self.plan.add(snapshot, &mut values, things);
        }
        values
    }

    /// The rise per second of `figure` to `value`, read now — `at`, as
    /// reading `number` — from the reading it is measured from: the oldest
    /// the window keeps, or the first since the figure was first read or
    /// last fell. None where no reading before this one holds it.
    fn measure(&mut self, figure: usize, value: u64, (at, number): (i64, u64)) -> Option<f64> {
        let since = &mut self.since[figure];
        let newest = self
            .readings
            .back()
            .filter(|newest| newest.number >= *since);
        if newest.is_some_and(|newest| value < newest.values[figure]) {
            *since = number;
            return None;
        }
        let first = match self.readings.front() {
            Some(oldest) if oldest.number >= *since => 0,
            _ => self
                .readings
                .partition_point(|reading| reading.number < *since),
        };
        let from = self.readings.get(first)?;
        per_second(value.saturating_sub(from.values[figure]), at - from.at)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::health::Health;
    use crate::snapshot::{Count, HealthRecord};
    use crate::topology::draw::members;
    use crate::topology::party::{self, Party};
    use crate::topology::{Origin, Pattern, TopologyLink, TopologyNode};

    use configure::fixture::test_cluster;

    const SECOND: i64 = 1_000_000_000;

    fn root() -> String {
        test_cluster().scope()
    }

    fn node() -> String {
        test_cluster().node(0).name.clone()
    }

    fn count(scope: String, counted: Counted, value: u64) -> Count {
        Count {
            scope,
            counted,
            value,
            window_start_unix_nanos: 7,
            window_end_unix_nanos: 7,
            observed_unix_nanos: 7,
        }
    }

    /// The test cluster's first node, alive, receiving over `In` and
    /// sending over `Out`, having received `streams`, routed `journeys`
    /// and sent `messages`, drawn at `second` with `any` on each side.
    fn drawn(second: i64, (streams, journeys, messages): (u64, u64, u64)) -> (Topology, Snapshot) {
        let at = format!("{}/node/{}", root(), node());
        let mut snapshot = Snapshot::new();
        for leaf in ["system-process", "receive/In", "process/onward", "send/Out"] {
            snapshot.record_health(HealthRecord {
                scope: format!("{at}/{leaf}"),
                health: Health::Fine,
                severity: 0,
                evidence: "alive: said".to_string(),
                observed_unix_nanos: 7,
            });
        }
        for (leaf, counted, value) in [
            ("receive/In", Counted::Streams, streams),
            ("process", Counted::Journeys, journeys),
            ("send/Out", Counted::Messages, messages),
        ] {
            snapshot.record_count(count(format!("{at}/{leaf}"), counted, value));
        }
        let names = [node()];
        let names: Vec<&str> = names.iter().map(String::as_str).collect();
        let mut topology = Topology {
            source: String::new(),
            observed_unix_nanos: second * SECOND,
            nodes: members(&snapshot, &root(), &names),
            links: Vec::new(),
        };
        party::draw(
            &snapshot,
            &root(),
            &names,
            &[Party::everywhere("any")],
            &mut topology,
        );
        (topology, snapshot)
    }

    fn activity(topology: &Topology, id: &str) -> f64 {
        topology
            .nodes
            .iter()
            .find(|node| node.id == id)
            .unwrap_or_else(|| panic!("{id} is drawn"))
            .activity
    }

    fn rate(topology: &Topology, id: &str) -> f64 {
        topology
            .links
            .iter()
            .find(|link| link.id == id)
            .unwrap_or_else(|| panic!("{id} is drawn"))
            .rate
    }

    fn near(said: f64, expected: f64) -> bool {
        (said - expected).abs() < 1e-9
    }

    #[test]
    fn a_burst_keeps_moving_through_the_rounds_after_it_and_stops_once_it_left_the_window() {
        let mut window = Window::new();
        let into = format!("party/sending/any/{}", node());
        let mut rates = Vec::new();
        for second in 0..=40 {
            let received = if second < 5 { 0 } else { 3 };
            let (mut topology, snapshot) = drawn(second, (received, received, 0));
            window.state(&mut topology, &snapshot);
            rates.push(rate(&topology, &into));
        }
        assert!(near(rates[0], 0.0), "nothing to measure from yet");
        assert!(near(rates[4], 0.0), "nothing moved yet");
        assert!(near(rates[6], 0.5), "three in six seconds: {}", rates[6]);
        assert!(
            rates[6..35].iter().all(|rate| *rate > 0.0),
            "a round that carried nothing still moves: {rates:?}"
        );
        assert!(near(rates[30], 0.1), "three in the half minute");
        assert!(
            rates[35..].iter().all(|rate| near(*rate, 0.0)),
            "the burst left the window: {rates:?}"
        );
    }

    #[test]
    fn a_thing_is_as_active_as_its_busiest_stage_and_idle_when_nothing_moved() {
        let mut window = Window::new();
        let (mut first, snapshot) = drawn(0, (0, 0, 0));
        window.state(&mut first, &snapshot);
        assert!(
            first.nodes.iter().all(|node| near(node.activity, 0.0)),
            "alive, and nothing moved: {:?}",
            first.nodes
        );
        let (mut later, snapshot) = drawn(2, (20, 20, 4));
        window.state(&mut later, &snapshot);
        let at = format!("node/{}", node());
        // Twenty received and routed are twenty that moved through it,
        // never forty.
        for (id, expected) in [
            (at.clone(), 10.0),
            (format!("{at}/receive"), 10.0),
            (format!("{at}/receive/In"), 10.0),
            (format!("{at}/process"), 10.0),
            (format!("{at}/send"), 2.0),
            ("cluster".to_string(), 10.0),
            // Nothing is counted beneath a Party: what its links carried.
            ("party/sending/any".to_string(), 10.0),
            ("party/receiving/any".to_string(), 2.0),
        ] {
            let said = activity(&later, &id);
            assert!(near(said, expected), "{id}: {said}");
        }

        // Still alive, and nothing more moved: idle once the window passed.
        let (mut still, snapshot) = drawn(2 + SPAN / SECOND + 1, (20, 20, 4));
        window.state(&mut still, &snapshot);
        assert!(near(activity(&still, &at), 0.0));
    }

    #[test]
    fn the_cluster_moves_with_its_nodes_and_never_with_a_test_run_at_the_cluster_itself() {
        // The Essentials run scenarios in its own process beside the nodes,
        // counted at `<root>/<test>`: the cluster's mood is read beneath its
        // nodes, and what moved through it is too.
        let in_process = |snapshot: &mut Snapshot, value: u64| {
            for leaf in ["round-trip/send/tcp", "daily-backlog"] {
                let scope = format!("{}/{leaf}", root());
                snapshot.record_count(count(scope, Counted::Streams, value));
            }
        };
        let mut window = Window::new();
        let (mut first, mut snapshot) = drawn(0, (0, 0, 0));
        in_process(&mut snapshot, 0);
        window.state(&mut first, &snapshot);
        let (mut later, mut snapshot) = drawn(2, (0, 0, 0));
        in_process(&mut snapshot, 16_980);
        window.state(&mut later, &snapshot);
        assert!(near(activity(&later, "cluster"), 0.0), "{later:?}");

        let (mut busy, mut snapshot) = drawn(4, (6, 0, 0));
        in_process(&mut snapshot, 33_960);
        window.state(&mut busy, &snapshot);
        assert!(near(activity(&busy, "cluster"), 1.5), "six in four seconds");
    }

    #[test]
    fn a_gauge_a_test_keeps_beneath_a_node_never_moves_it_and_its_stages_do() {
        // The Essentials' daily backlog counts its directory's depth as
        // Messages beneath the node, on no stage: it climbs with the queue
        // and falls as the queue drains, and never moved through the node.
        let mut window = Window::new();
        let at = format!("node/{}", node());
        let backlog = format!("{}/{at}/daily-backlog", root());
        for (second, moved, depth) in [(0, 0, 0), (2, 4, 40), (4, 8, 10)] {
            let (mut topology, mut snapshot) = drawn(second, (moved, moved, 0));
            snapshot.record_count(count(backlog.clone(), Counted::Messages, depth));
            window.state(&mut topology, &snapshot);
            if second > 0 {
                for id in [at.as_str(), "cluster"] {
                    let said = activity(&topology, id);
                    assert!(near(said, 2.0), "{id} at {second}: {said}");
                }
            }
        }
    }

    /// A link `from` one drawn thing `to` another, having carried `volume`.
    fn link(id: &str, (from, to): (&str, &str), volume: u64) -> TopologyLink {
        TopologyLink {
            id: id.to_string(),
            from: from.to_string(),
            to: to.to_string(),
            pattern: Pattern::SendReceive,
            origin: Origin::Both,
            protocol: String::new(),
            state: Health::Fine,
            volume,
            rate: 0.0,
            latency_ms: 0.0,
            progress: 0.0,
            attempts: 0,
            evidence: String::new(),
        }
    }

    #[test]
    fn what_passes_through_a_stage_moves_it_once_never_coming_in_and_again_going_out() {
        let stage = TopologyNode {
            id: "b".to_string(),
            parent: String::new(),
            label: "b".to_string(),
            kind: NodeKind::default(),
            scope: format!("{}/nothing-counted", root()),
            state: Health::Fine,
            origin: Origin::Both,
            load: 0.0,
            activity: 0.0,
            evidence: String::new(),
        };
        let mut window = Window::new();
        for (second, carried) in [(0, 0), (2, 8)] {
            let mut topology = Topology {
                observed_unix_nanos: second * SECOND,
                nodes: vec![stage.clone()],
                links: vec![
                    link("in", ("a", "b"), carried),
                    link("out", ("b", "c"), carried),
                    link("out too", ("b", "d"), carried / 2),
                ],
                ..Topology::default()
            };
            window.state(&mut topology, &Snapshot::new());
            if second == 2 {
                // Four a second came in and six went out: six moved through
                // it, never ten.
                let said = activity(&topology, "b");
                assert!(near(said, 6.0), "{said}");
            }
        }
    }

    /// Links between `a` and `b`, by id and volume, observed at `second`.
    fn links_at(second: i64, links: &[(&str, u64)]) -> Topology {
        Topology {
            observed_unix_nanos: second * SECOND,
            links: links
                .iter()
                .map(|(id, volume)| link(id, ("a", "b"), *volume))
                .collect(),
            ..Topology::default()
        }
    }

    #[test]
    fn a_publisher_that_started_over_is_measured_from_its_new_start_and_no_interval_is_no_rate() {
        let snapshot = Snapshot::new();
        let mut window = Window::new();
        for (second, volume) in [(0, 50), (1, 2), (3, 8)] {
            let mut topology = links_at(second, &[("fell", volume), ("steady", 4)]);
            window.state(&mut topology, &snapshot);
            if second == 3 {
                assert!(near(rate(&topology, "fell"), 3.0), "six in two seconds");
                assert!(near(rate(&topology, "steady"), 0.0));
            }
        }
        // Its new count past the oldest reading the window holds: still
        // measured from its new start, 150 in twenty seconds, never 50 in
        // thirty.
        let mut restarted = Window::new();
        for (second, volume) in [(0, 100), (10, 0), (30, 150)] {
            let mut topology = links_at(second, &[("restarted", volume)]);
            restarted.state(&mut topology, &snapshot);
            if second == 30 {
                let said = rate(&topology, "restarted");
                assert!(near(said, 7.5), "{said}");
            }
        }
        let drawn_as = |volume: u64| TopologyLink {
            rate: 2.5,
            ..link("new", ("a", "b"), volume)
        };
        let mut again = Topology {
            links: vec![drawn_as(9)],
            ..links_at(3, &[])
        };
        window.state(&mut again, &snapshot);
        assert!(near(rate(&again, "new"), 2.5), "drawn for the first time");
        let mut twice = Topology {
            links: vec![drawn_as(12)],
            ..links_at(3, &[])
        };
        window.state(&mut twice, &snapshot);
        assert!(near(rate(&twice, "new"), 2.5), "no time between the two");
    }

    #[test]
    fn a_clock_that_steps_back_starts_the_window_over_and_the_next_round_states_a_rate() {
        // A time sync or a restored virtual machine: every reading is
        // stamped later than the new one, and none may be measured from.
        let snapshot = Snapshot::new();
        let mut window = Window::new();
        let mut said = Vec::new();
        for (second, volume) in [(70, 10), (75, 20), (15, 30), (17, 34), (19, 38)] {
            let mut topology = links_at(second, &[("on", volume)]);
            window.state(&mut topology, &snapshot);
            said.push(rate(&topology, "on"));
        }
        assert!(near(said[1], 2.0), "{said:?}");
        assert!(near(said[2], 0.0), "nothing to measure from: {said:?}");
        assert!(near(said[3], 2.0), "from the reading after the step");
        assert!(near(said[4], 2.0), "{said:?}");
        let kept = &window.readings;
        assert!(
            kept.iter().all(|reading| reading.at <= 19 * SECOND),
            "{kept:?}"
        );
    }

    #[test]
    fn a_drawing_of_another_shape_carries_what_was_read_of_each_figure_still_drawn() {
        // A link drawn for the first time, and a Location counted for the
        // first time: what was drawn before is still measured over the
        // whole window.
        let mut window = Window::new();
        let into = format!("party/sending/any/{}", node());
        for (second, received) in [(0, 0), (5, 10)] {
            let (mut topology, snapshot) = drawn(second, (received, 0, 0));
            window.state(&mut topology, &snapshot);
        }
        let (mut grown, mut snapshot) = drawn(10, (20, 0, 0));
        grown.links.push(TopologyLink {
            rate: 2.5,
            ..link("new", ("a", "b"), 7)
        });
        let at = format!("{}/node/{}/receive/Second", root(), node());
        snapshot.record_count(count(at, Counted::Streams, 30));
        window.state(&mut grown, &snapshot);
        // Twenty over `In` in the ten seconds, which the Party's link
        // carries; and thirty more over the new Location, beneath the node:
        // fifty through it.
        assert!(near(rate(&grown, &into), 2.0), "{grown:?}");
        assert!(near(rate(&grown, "new"), 2.5), "drawn for the first time");
        let at = format!("node/{}", node());
        assert!(near(activity(&grown, &at), 5.0), "{grown:?}");
    }

    #[test]
    fn a_publication_every_quarter_second_keeps_about_one_reading_a_second() {
        // `xmip-service` publishes every quarter of a second, and at once
        // after every order or act: what the window holds stays bounded.
        let snapshot = Snapshot::new();
        let mut window = Window::new();
        let quarter = SECOND / 4;
        let mut last = 0.0;
        for round in 0..=(4 * 120) {
            let mut topology = Topology {
                observed_unix_nanos: round * quarter,
                links: vec![link(
                    "busy",
                    ("a", "b"),
                    u64::try_from(round).expect("small"),
                )],
                ..Topology::default()
            };
            window.state(&mut topology, &snapshot);
            last = rate(&topology, "busy");
        }
        let kept = window.readings.len();
        let most = usize::try_from(SPAN / STEP).expect("small") + 2;
        assert!(kept <= most, "{kept} readings kept");
        // Four a second, measured over the half minute or a step more.
        assert!((last - 4.0).abs() < 0.2, "{last}");
        // A figure no longer drawn is forgotten.
        window.state(&mut links_at(121, &[]), &snapshot);
        assert_eq!(window.since, [] as [u64; 0]);
        assert!(
            window
                .readings
                .iter()
                .all(|reading| reading.values.is_empty())
        );
    }
}
