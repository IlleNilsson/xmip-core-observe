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
//! after it — the Playground's transports, a service node's Locations. A
//! drawn thing above its leaves is Fine or Holding (`Health::rolled`,
//! ADR-0041).
//!
//! Drawn here once, for every publisher: the Playground's roll draws its
//! cluster through it and adds its handoffs and its shared store, and a
//! running node (`xmip-core-runtime`'s `Running::publication`) draws itself
//! through it; the Parties outside the nodes are [`super::party`]'s. Until
//! 2026-09-30 the Playground held this drawing, and no other publisher drew
//! a topology at all.

use std::collections::BTreeSet;

use node::{Capability, Stage};

use crate::health::Health;
use crate::scope::Scope;
use crate::snapshot::{HealthRecord, Snapshot};
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
    format!("node/{name}")
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

/// Configured, and observed too once something reported on it.
#[must_use]
pub fn origin(snapshot: &Snapshot, scope: &str) -> Origin {
    if snapshot.health(scope).is_empty() {
        Origin::Configured
    } else {
        Origin::Both
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

/// Whether the record at the node's [`system_process`] says it runs.
#[must_use]
pub fn alive(snapshot: &Snapshot, node_scope: &str) -> bool {
    worst(snapshot, &system_process(node_scope))
        .is_some_and(|record| record.evidence.starts_with(ALIVE))
}

/// The cluster at `root` and every node named, each followed by its stages
/// and their endpoints, in the order named.
#[must_use]
pub fn members(snapshot: &Snapshot, root: &str, names: &[&str]) -> Vec<TopologyNode> {
    let mut drawn = vec![cluster(snapshot, root, names)];
    for name in names {
        let scope = format!("{root}/node/{name}");
        drawn.push(node(snapshot, name, &scope));
        drawn.extend(stages(snapshot, name, &scope));
    }
    drawn
}

/// The cluster: its mood the rollup over its nodes, its activity the share
/// of them running.
#[must_use]
pub fn cluster(snapshot: &Snapshot, root: &str, names: &[&str]) -> TopologyNode {
    let (state, evidence) = drawn(worst(snapshot, &format!("{root}/node")).as_ref(), root);
    let running = names
        .iter()
        .filter(|name| alive(snapshot, &format!("{root}/node/{name}")))
        .count();
    TopologyNode {
        id: CLUSTER.to_string(),
        parent: String::new(),
        label: label(root).to_string(),
        kind: NodeKind::Cluster,
        scope: root.to_string(),
        state,
        origin: Origin::Configured,
        load: 0.0,
        activity: fraction(running, names.len()),
        evidence,
    }
}

/// One node of the cluster, the System Process it is (ADR-0028 clause 2).
#[must_use]
pub fn node(snapshot: &Snapshot, name: &str, scope: &str) -> TopologyNode {
    let (state, evidence) = drawn(worst(snapshot, scope).as_ref(), scope);
    TopologyNode {
        id: node_id(name),
        parent: CLUSTER.to_string(),
        label: name.to_string(),
        kind: NodeKind::Node,
        scope: scope.to_string(),
        state,
        origin: origin(snapshot, scope),
        load: 0.0,
        activity: if alive(snapshot, scope) { 1.0 } else { 0.0 },
        evidence,
    }
}

/// The stage nodes of the node called `name`, each followed by its
/// endpoints: every stage the node **declared** it can serve, drawn before it
/// has reported, and any stage it has reported on. Its name says nothing
/// here (ADR-0056).
#[must_use]
pub fn stages(snapshot: &Snapshot, name: &str, scope: &str) -> Vec<TopologyNode> {
    let declared = declared(snapshot, scope);
    let mut drawn = Vec::new();
    for stage in Stage::ALL {
        let at = format!("{scope}/{}", stage.name());
        if !declared.can(stage) && snapshot.health(&at).is_empty() {
            continue;
        }
        let id = format!("{}/{}", node_id(name), stage.name());
        drawn.push(part(
            snapshot,
            &id,
            &node_id(name),
            stage.name(),
            NodeKind::Stage,
            &at,
        ));
        // A process stage touches no transport: it has no endpoints.
        if stage != Stage::Process {
            for endpoint in endpoints(snapshot, &at) {
                drawn.push(part(
                    snapshot,
                    &format!("{id}/{endpoint}"),
                    &id,
                    &endpoint,
                    NodeKind::Endpoint,
                    &format!("{at}/{endpoint}"),
                ));
            }
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
fn declared(snapshot: &Snapshot, scope: &str) -> Capability {
    let at = crate::capability::scope(scope);
    snapshot
        .health(&at)
        .into_iter()
        .find(|record| record.scope == at)
        .and_then(|record| {
            crate::capability::declared(&record.scope, &record.evidence)
                .and_then(|(_, said)| said.ok())
        })
        .unwrap_or_else(Capability::none)
}

/// The endpoints a stage has reported on: the segment after the stage in
/// every scope beneath it, once each, in order.
fn endpoints(snapshot: &Snapshot, stage_scope: &str) -> BTreeSet<String> {
    let beneath = format!("{stage_scope}/");
    snapshot
        .health(stage_scope)
        .iter()
        .filter_map(|record| record.scope.strip_prefix(&beneath))
        .filter_map(|rest| rest.split('/').next())
        .filter(|endpoint| !endpoint.is_empty())
        .map(str::to_string)
        .collect()
}

/// A stage or an endpoint: its own mood, or the rollup over the worst
/// beneath its scope (ADR-0041).
fn part(
    snapshot: &Snapshot,
    id: &str,
    parent: &str,
    label: &str,
    kind: NodeKind,
    scope: &str,
) -> TopologyNode {
    let (state, evidence) = drawn(worst(snapshot, scope).as_ref(), scope);
    TopologyNode {
        id: id.to_string(),
        parent: parent.to_string(),
        label: label.to_string(),
        kind,
        scope: scope.to_string(),
        state,
        origin: origin(snapshot, scope),
        load: 0.0,
        activity: 0.0,
        evidence,
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

        let drawn = members(&snapshot, &root, &[name]);
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
        assert!((drawn[0].activity - 1.0).abs() < f64::EPSILON, "alive");
        assert_eq!(drawn[3].state, Health::Stressed, "the endpoint's own");
        assert_eq!(drawn[2].state, Health::Holding, "a parent rolls up");
        assert_eq!(
            drawn[4].origin,
            Origin::Configured,
            "declared, not reported"
        );
        assert_eq!(system_process(&scope), format!("{scope}/system-process"));
    }
}
