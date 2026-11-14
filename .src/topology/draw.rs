//! A cluster drawn from what its nodes published: the cluster, one node per
//! name, each node's stages of the message path and the endpoints beneath a
//! receive or a send stage (ADR-0052, amendment 2026-09-14, ruling 3). The
//! picture is the owner's, 2026-09-19: *cluster, nodes, receive, process,
//! send*.
//!
//! Nothing here is inferred from a socket or from a name. A node is drawn
//! because its publisher names it; a stage because the node declared it
//! (ADR-0056: its record at [`crate::capability::scope`]) or reported on it;
//! an endpoint because a record sits beneath the stage, one per segment
//! after it — the Essentials' transports, a service node's Locations. A
//! drawn thing above its leaves is Fine or Holding (`Health::rolled`,
//! ADR-0041). Everything is drawn still, its activity zero: what moved
//! through it is stated by its publisher's [`super::window::Window`], never
//! by whether it is alive (ADR-0052, amendment 2026-11-10).
//!
//! Drawn here once, for every publisher: the Essentials' roll draws its
//! cluster through it and adds its handoffs and its shared store, and a
//! running node (`xmip-core-runtime`'s `Running::publication`) draws itself
//! through it; the Parties outside the nodes are [`super::party`]'s. Until
//! 2026-09-30 the Essentials held this drawing, and no other publisher drew
//! a topology at all.

use std::collections::BTreeSet;

use node::scope::{NODE, Scope};
use node::{Capability, Stage};

use crate::health::Health;
use crate::snapshot::{Beneath, HealthRecord, Snapshot};
use crate::topology::relate::Member;
use crate::topology::{NodeKind, Origin, TopologyNode};

/// The id of the cluster, the one drawn thing with no parent.
pub const CLUSTER: &str = "cluster";

/// The leaf beneath a node its System Process's record sits at (ADR-0028
/// clause 2, ADR-0053): whoever runs the process says there whether it is
/// alive.
const SYSTEM_PROCESS: &str = "system-process";

/// The word a System Process's record opens with while it runs.
pub const ALIVE: &str = "alive";

/// Where the System Process of the node at `node_scope` says whether it is
/// alive: a record whose evidence opens with [`ALIVE`] while it runs.
#[must_use]
pub fn system_process(node_scope: &str) -> String {
    format!("{}/{SYSTEM_PROCESS}", node_scope.trim_end_matches('/'))
}

/// The id of the node called `name`.
#[must_use]
pub fn node_id(name: &str) -> String {
    joined(&["node/", name])
}

/// `parts` written one after another, in one allocation: what a drawing
/// writes its ids and scopes with, thousands a round, where `format!` grows
/// its text as it writes and allocates several times for one.
#[must_use]
pub fn joined(parts: &[&str]) -> String {
    let mut text = String::with_capacity(parts.iter().map(|part| part.len()).sum());
    for part in parts {
        text.push_str(part);
    }
    text
}

/// The last segment of a scope: what a thing is called.
#[must_use]
pub fn label(scope: &str) -> &str {
    scope.rsplit('/').next().unwrap_or(scope)
}

/// The worst record at or beneath `scope`, if anything was published there.
#[must_use]
pub fn worst(snapshot: &Snapshot, scope: &str) -> Option<HealthRecord> {
    snapshot.health(scope).into_iter().next()
}

/// What a drawn thing at `scope` shows (ADR-0041): the mood of the record
/// at the scope itself, or — for a parent — `Health::rolled` over the worst
/// record beneath it, Fine or Holding; the evidence is the worst record's
/// either way, so a Holding cluster says what it is holding on.
#[must_use]
pub fn drawn(record: Option<&HealthRecord>, scope: &str) -> (Health, String) {
    let (health, evidence) = mood(record);
    match record {
        Some(record) if Scope::new(&record.scope) != Scope::new(scope) => {
            (health.rolled(), evidence)
        }
        _ => (health, evidence),
    }
}

/// The mood and the evidence of a record, or what an empty scope says: what
/// a link between two things shows, which is no parent of anything.
#[must_use]
pub fn mood(record: Option<&HealthRecord>) -> (Health, String) {
    record.map_or_else(
        || (Health::Working, "nothing published yet".to_string()),
        |record| (record.health, record.evidence.clone()),
    )
}

/// Configured, and observed too once something reported on it: `worst`,
/// the worst record at or beneath the drawn thing's scope, where any is.
#[must_use]
pub const fn origin(worst: Option<&HealthRecord>) -> Origin {
    if worst.is_some() {
        Origin::Both
    } else {
        Origin::Configured
    }
}

/// `part` of `whole`, from zero to one; zero of nothing.
#[must_use]
#[allow(clippy::cast_precision_loss)] // a share is a figure to read, not a count
pub fn fraction(part: usize, whole: usize) -> f64 {
    if whole == 0 {
        0.0
    } else {
        part as f64 / whole as f64
    }
}

/// The cluster at `root` and every member, each saying the roles it
/// declares and its place in `[storage] nodes` as its caller knows them —
/// the roll from the cluster's file, a running node from itself — and
/// followed by its stages and their endpoints, in the order given. The
/// snapshot's records are ordered once, each member's searched once, and
/// every scope of a member asked of its own ([`Beneath`]).
#[must_use]
pub fn members(snapshot: &Snapshot, root: &str, members: &[Member]) -> Vec<TopologyNode> {
    let beneath = Beneath::of(snapshot);
    let mut drawn = vec![cluster(&beneath, root)];
    for member in members {
        let scope = joined(&[root, "/", NODE, "/", &member.node]);
        let own = beneath.within(&scope);
        drawn.push(TopologyNode {
            roles: member.words(),
            place: member.storage_place,
            ..node(&own, &member.node, &scope)
        });
        drawn.extend(stages(&own, &member.node, &scope));
    }
    drawn
}

/// What is placed beneath the stage `stage` of the node at `node_scope`, by
/// name, once each: each Location or Port it reported on beneath receive and
/// send, and beneath process each Subscription the node published it routes
/// by — what its stage is drawn holding, and what a relation says is placed
/// on it ([`super::relate`]).
#[must_use]
pub fn placed(beneath: &Beneath<'_>, node_scope: &str, stage: Stage) -> BTreeSet<String> {
    if stage == Stage::Process {
        routed_by(beneath.snapshot(), node_scope)
    } else {
        endpoints(beneath, &joined(&[node_scope, "/", stage.name()]))
    }
}

/// Where the cluster at `root` is read, its mood and what moved through it
/// alike: beneath its nodes, `<root>/node`, never what a test run at the
/// cluster itself recorded.
#[must_use]
pub fn node_rollup(root: &str) -> String {
    format!("{}/{NODE}", root.trim_end_matches('/'))
}

/// The cluster: its mood the rollup over its nodes.
#[must_use]
pub fn cluster(beneath: &Beneath<'_>, root: &str) -> TopologyNode {
    let (state, evidence) = drawn(beneath.worst(&node_rollup(root)), root);
    TopologyNode {
        id: CLUSTER.to_string(),
        parent: String::new(),
        label: label(root).to_string(),
        kind: NodeKind::Cluster,
        scope: root.to_string(),
        state,
        origin: Origin::Configured,
        load: 0.0,
        activity: 0.0,
        evidence,
        roles: String::new(),
        place: None,
    }
}

/// One node of the cluster, the System Process it is (ADR-0028 clause 2).
#[must_use]
pub fn node(beneath: &Beneath<'_>, name: &str, scope: &str) -> TopologyNode {
    let worst = beneath.worst(scope);
    let (state, evidence) = drawn(worst, scope);
    TopologyNode {
        id: node_id(name),
        parent: CLUSTER.to_string(),
        label: name.to_string(),
        kind: NodeKind::Node,
        scope: scope.to_string(),
        state,
        origin: origin(worst),
        load: 0.0,
        activity: 0.0,
        evidence,
        roles: String::new(),
        place: None,
    }
}

/// The stage nodes of the node called `name`, each followed by its
/// endpoints: every stage the node **declared** it can serve, drawn before it
/// has reported, and any stage it has reported on. Its name says nothing
/// here (ADR-0056).
#[must_use]
pub fn stages(beneath: &Beneath<'_>, name: &str, scope: &str) -> Vec<TopologyNode> {
    let declared = declared(beneath, scope);
    let mut node = None;
    let mut drawn = Vec::new();
    for stage in Stage::ALL {
        if !declared.can(stage) && !beneath.holds_beneath(scope, stage.name()) {
            continue;
        }
        let node: &str = node.get_or_insert_with(|| node_id(name));
        let (at, id) = (
            joined(&[scope, "/", stage.name()]),
            joined(&[node, "/", stage.name()]),
        );
        // Labeled by its title, Work Process for the process stage; the id
        // and the scope keep the word (ADR-0027, amendment 2026-11-11).
        let kind = (stage.title().to_string(), NodeKind::Stage);
        drawn.push(part(
            beneath.worst(&at),
            (id.clone(), node),
            kind,
            at.clone(),
        ));
        // Beneath receive and send each Location or Port it reported on;
        // beneath the process stage each Subscription the node published
        // it routes by, which a Receive Location feeds, and nothing else
        // its scopes hold (ADR-0018, amendment 2026-11-11).
        for endpoint in placed(beneath, scope, stage) {
            let within = joined(&[&at, "/", &endpoint]);
            let ids = (joined(&[&id, "/", &endpoint]), id.as_str());
            let worst = beneath.worst(&within);
            drawn.push(part(worst, ids, (endpoint, NodeKind::Endpoint), within));
        }
    }
    drawn
}

/// What the node published at `<scope>/capability`: what it declared it can
/// do, its roles saying which stages it serves. Nothing published is
/// nothing declared — the node has yet to say, and only what it reports is
/// drawn. A record naming a word that is no role is refused and draws no
/// declared stage either; the record itself stays on the node's `capability`
/// scope in the publisher's words.
fn declared(beneath: &Beneath<'_>, scope: &str) -> Capability {
    let at = crate::capability::scope(scope);
    beneath
        .records(&at)
        .find(|record| record.scope == at)
        .and_then(|record| Capability::from_evidence(&record.evidence).ok())
        .unwrap_or_else(Capability::none)
}

/// The endpoints a stage has reported on: the segment after the stage in
/// every scope beneath it, once each, in order.
fn endpoints(beneath: &Beneath<'_>, stage_scope: &str) -> BTreeSet<String> {
    let under = joined(&[stage_scope, "/"]);
    beneath
        .records(stage_scope)
        .filter_map(|record| record.scope.strip_prefix(&under))
        .filter_map(|rest| rest.split('/').next())
        .filter(|endpoint| !endpoint.is_empty())
        .map(str::to_string)
        .collect()
}

/// The Subscriptions the node at `scope` published it routes by, by name,
/// once each: what its process stage is drawn holding (`Subscription`).
fn routed_by(snapshot: &Snapshot, scope: &str) -> BTreeSet<String> {
    snapshot
        .subscriptions()
        .filter(|subscription| Scope::new(&subscription.node) == Scope::new(scope))
        .map(|subscription| subscription.name.clone())
        .collect()
}

/// A stage or an endpoint at `scope`, its `id` beneath `parent`, labelled as
/// `kind`: its own mood, or the rollup over `worst`, the worst record
/// beneath its scope (ADR-0041).
fn part(
    worst: Option<&HealthRecord>,
    (id, parent): (String, &str),
    (label, kind): (String, NodeKind),
    scope: String,
) -> TopologyNode {
    let (state, evidence) = drawn(worst, &scope);
    TopologyNode {
        id,
        parent: parent.to_string(),
        label,
        kind,
        scope,
        state,
        origin: origin(worst),
        load: 0.0,
        activity: 0.0,
        evidence,
        roles: String::new(),
        place: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use configure::fixture::test_cluster;
    use node::NodeRole;

    fn record(scope: &str, health: Health, evidence: &str) -> HealthRecord {
        HealthRecord {
            scope: scope.to_string(),
            health,
            severity: if health == Health::Fine { 0 } else { 60 },
            evidence: evidence.to_string(),
            observed_unix_nanos: 7,
        }
    }

    #[test]
    fn a_node_draws_what_it_declared_and_an_endpoint_per_segment_beneath_a_stage() {
        let cluster = test_cluster();
        let (root, scope, name) = (
            cluster.scope(),
            cluster.node_scope(0),
            cluster.node(0).name.as_str(),
        );
        let mut snapshot = Snapshot::new();
        let declares = Capability::of(&[NodeRole::Receiving, NodeRole::Processing]).evidence();
        for (leaf, health, evidence) in [
            ("capability", Health::Fine, declares.as_str()),
            ("system-process", Health::Fine, "alive: pid 7"),
            ("receive/In", Health::Stressed, "refused at its gate"),
            ("receive/In/authentication", Health::Fine, "said"),
        ] {
            snapshot.record_health(record(&format!("{scope}/{leaf}"), health, evidence));
        }

        let drawn = members(&snapshot, &root, &[Member::new(name, &[], None)]);
        let shape: Vec<(&str, &str, &str)> = drawn
            .iter()
            .map(|node| (node.id.as_str(), node.kind.word(), node.parent.as_str()))
            .collect();
        let (node, receive) = (format!("node/{name}"), format!("node/{name}/receive"));
        let (endpoint, process) = (format!("{receive}/In"), format!("{node}/process"));
        assert_eq!(
            shape,
            [
                ("cluster", "cluster", ""),
                (node.as_str(), "node", "cluster"),
                (receive.as_str(), "stage", node.as_str()),
                (endpoint.as_str(), "endpoint", receive.as_str()),
                (process.as_str(), "stage", node.as_str()),
            ]
        );
        // Alive, and drawn still: what moves is its window's to state.
        assert!(
            drawn
                .iter()
                .all(|drawn| drawn.activity.abs() < f64::EPSILON),
            "{drawn:?}"
        );
        assert_eq!(drawn[3].state, Health::Stressed, "the endpoint's own");
        assert_eq!(drawn[2].state, Health::Holding, "a parent rolls up");
        assert_eq!(
            drawn[4].origin,
            Origin::Configured,
            "declared, not reported"
        );
        assert_eq!(
            [drawn[2].label.as_str(), drawn[4].label.as_str()],
            ["Receive", "Work Process"],
            "a stage reads by its title, never a bare Process"
        );

        // A Subscription the node published is drawn beneath its process
        // stage, as a Location is beneath receive; whatever else a scope
        // beneath the stage holds — a transport's name — is not.
        for leaf in ["Invoices", "tcp"] {
            snapshot.record_health(record(&format!("{scope}/process/{leaf}"), Health::Fine, ""));
        }
        snapshot.record_subscription(crate::Subscription {
            node: scope.clone(),
            name: "Invoices".into(),
            ..crate::Subscription::default()
        });
        let drawn = members(&snapshot, &root, &[Member::new(name, &[], None)]);
        let beneath: Vec<&str> = drawn
            .iter()
            .filter(|one| one.parent == process)
            .map(|one| one.label.as_str())
            .collect();
        assert_eq!(beneath, ["Invoices"], "{drawn:?}");
        assert_eq!(system_process(&scope), format!("{scope}/system-process"));
    }
}
