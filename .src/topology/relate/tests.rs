use std::collections::{BTreeMap, BTreeSet};

use configure::fixture::test_cluster;
use node::{NodeRole, Stage};

use super::{Member, handed, listed, relations};
use crate::health::Health;
use crate::snapshot::{HealthRecord, Snapshot};
use crate::topology::{Origin, Relation, TopologyLink};

/// The test cluster's members, each declaring what its file says, the
/// Storage nodes listed in the order the file names them, `renamed` giving
/// each its name.
fn cluster(renamed: &dyn Fn(&str) -> String) -> Vec<Member> {
    let mut place = 0;
    test_cluster()
        .nodes
        .iter()
        .map(|node| {
            let roles: Vec<NodeRole> = node
                .roles
                .iter()
                .filter_map(|word| NodeRole::named(word))
                .collect();
            let listed = roles.contains(&NodeRole::Storage).then(|| {
                place += 1;
                place - 1
            });
            Member::new(&renamed(&node.name), &roles, listed)
        })
        .collect()
}

fn root() -> String {
    test_cluster().scope()
}

/// Each relation as its two ends' names and what it is.
fn related(links: &[TopologyLink]) -> BTreeSet<(String, String, Relation)> {
    let name = |id: &str| id.strip_prefix("node/").unwrap_or(id).to_string();
    links
        .iter()
        .map(|link| (name(&link.from), name(&link.to), link.relation))
        .collect()
}

/// The members declaring `role`.
fn declaring(members: &[Member], role: NodeRole) -> Vec<&str> {
    members
        .iter()
        .filter(|member| member.declares(role))
        .map(|member| member.node.as_str())
        .collect()
}

#[test]
fn every_member_is_related_by_its_roles_to_the_configured_primary() {
    let members = cluster(&|name| name.to_string());
    let links = relations(&root(), &members, &Snapshot::new());
    let said = related(&links);
    let storing = listed(&members);
    let (primary, standbys) = (storing[0].node.as_str(), &storing[1..]);
    for member in &members {
        let name = member.node.as_str();
        let writes = (name.to_string(), primary.to_string(), Relation::WritesInto);
        assert_eq!(
            said.contains(&writes),
            !member.declares(NodeRole::Storage),
            "{name} writes into the primary unless it stores"
        );
        let handed_work = (primary.to_string(), name.to_string(), Relation::HandsWorkTo);
        assert_eq!(
            said.contains(&handed_work),
            member.serves(Stage::Process) || member.serves(Stage::Send),
            "{name} is handed work where it processes or sends"
        );
    }
    for standby in standbys {
        let copied = (
            primary.to_string(),
            standby.node.clone(),
            Relation::CopiesTo,
        );
        assert!(said.contains(&copied), "{copied:?}");
        assert!(
            !said.iter().any(|(from, to, relation)| to == &standby.node
                && *relation == Relation::WritesInto
                && from != primary),
            "a standby is reached through copies to, never written into"
        );
    }
    for operating in declaring(&members, NodeRole::Operational) {
        for other in members.iter().filter(|other| other.node != operating) {
            let line = (
                operating.to_string(),
                other.node.clone(),
                Relation::OperatesAndMonitors,
            );
            assert!(said.contains(&line), "{line:?}");
        }
    }
    assert!(
        links
            .iter()
            .all(|link| link.origin == Origin::Configured && link.volume == 0),
        "configured until traffic crosses it"
    );
    let ids: BTreeSet<&str> = links.iter().map(|link| link.id.as_str()).collect();
    assert_eq!(ids.len(), links.len(), "each relation its own id");
}

/// Nothing reads a name: every node renamed — the receiving nodes given the
/// Storage nodes' names and the Storage nodes the receiving nodes', the
/// rest strings with no letters to read — draws the same relations, read
/// back through the renaming.
#[test]
fn a_renamed_cluster_is_related_the_same() {
    let members = cluster(&|name| name.to_string());
    let receiving = declaring(&members, NodeRole::Receiving);
    let storing = declaring(&members, NodeRole::Storage);
    let mut renaming: BTreeMap<String, String> = BTreeMap::new();
    for (one, other) in receiving.iter().zip(storing.iter()) {
        renaming.insert((*one).to_string(), (*other).to_string());
        renaming.insert((*other).to_string(), (*one).to_string());
    }
    for (at, member) in members.iter().enumerate() {
        renaming
            .entry(member.node.clone())
            .or_insert_with(|| format!("q{}z-{at}", 97 - at));
    }
    let back: BTreeMap<String, String> = renaming
        .iter()
        .map(|(old, new)| (new.clone(), old.clone()))
        .collect();
    let renamed = cluster(&|name| renaming[name].clone());

    let first = related(&relations(&root(), &members, &Snapshot::new()));
    let again: BTreeSet<(String, String, Relation)> =
        related(&relations(&root(), &renamed, &Snapshot::new()))
            .into_iter()
            .map(|(from, to, relation)| (back[&from].clone(), back[&to].clone(), relation))
            .collect();
    assert_eq!(first, again);
}

/// Two nodes keep their names and swap their roles: their relations swap.
#[test]
fn swapped_roles_swap_relations() {
    let members = cluster(&|name| name.to_string());
    let one = declaring(&members, NodeRole::Receiving)[0].to_string();
    let other = declaring(&members, NodeRole::Processing)[0].to_string();
    let swapped: Vec<Member> = members
        .iter()
        .map(|member| {
            let roles = if member.node == one {
                vec![NodeRole::Processing]
            } else if member.node == other {
                vec![NodeRole::Receiving]
            } else {
                member.roles.clone()
            };
            Member::new(&member.node, &roles, member.storage_place)
        })
        .collect();
    let swap = |name: &String| {
        if *name == one {
            other.clone()
        } else if *name == other {
            one.clone()
        } else {
            name.clone()
        }
    };
    let first = related(&relations(&root(), &members, &Snapshot::new()));
    let again: BTreeSet<(String, String, Relation)> =
        related(&relations(&root(), &swapped, &Snapshot::new()))
            .into_iter()
            .map(|(from, to, relation)| (swap(&from), swap(&to), relation))
            .collect();
    assert_eq!(first, again);
}

fn record(scope: String, health: Health, evidence: &str) -> HealthRecord {
    HealthRecord {
        scope,
        health,
        severity: if health == Health::Fine { 0 } else { 60 },
        evidence: evidence.to_string(),
        observed_unix_nanos: 7,
    }
}

/// The configured lines go to the first listed whatever was promoted, an
/// idle processing node's line says nothing is placed on it, and a node that
/// cannot reach Storage has a troubled line, saying why.
#[test]
fn the_configured_primary_holds_through_a_promotion_and_a_line_says_its_ends_trouble() {
    let members = cluster(&|name| name.to_string());
    let storing = listed(&members);
    let (primary, standby) = (&storing[0].node, &storing[1].node);
    let processing = declaring(&members, NodeRole::Processing)[0];
    let receiving = declaring(&members, NodeRole::Receiving)[0];
    let mut snapshot = Snapshot::new();
    let at = |name: &str, leaf: &str| format!("{}/node/{name}/{leaf}", root());
    snapshot.record_health(record(at(standby, "storage"), Health::Fine, "primary of 2"));
    snapshot.record_health(record(
        at(receiving, "storage"),
        Health::Exhausted,
        "no Storage node answers",
    ));
    snapshot.record_health(record(at(receiving, "receive/In"), Health::Fine, "serving"));
    let links = relations(&root(), &members, &snapshot);
    let line = |from: &str, to: &str, relation: Relation| {
        links
            .iter()
            .find(|link| {
                link.from == format!("node/{from}")
                    && link.to == format!("node/{to}")
                    && link.relation == relation
            })
            .unwrap_or_else(|| panic!("{from} {} {to}", relation.name()))
    };
    let idle = line(processing, primary, Relation::WritesInto);
    assert!(
        idle.evidence.contains("Work Process: none placed"),
        "{}",
        idle.evidence
    );
    assert!(
        idle.evidence.contains(&format!("{standby}, a standby")),
        "{}",
        idle.evidence
    );
    let troubled = line(receiving, primary, Relation::WritesInto);
    assert_eq!(troubled.state, Health::Exhausted);
    assert!(
        troubled.evidence.contains("Receive: In")
            && troubled.evidence.ends_with("no Storage node answers"),
        "{}",
        troubled.evidence
    );
    assert!(
        links
            .iter()
            .all(|link| !(link.to == format!("node/{standby}")
                && link.relation == Relation::WritesInto)),
        "nothing is written into the standby the snapshot says was promoted"
    );
}

#[test]
fn work_is_handed_from_each_that_receives_to_each_that_processes_and_on() {
    let members = cluster(&|name| name.to_string());
    let pairs = handed(&members);
    assert!(!pairs.is_empty());
    for (from, from_stage, to, to_stage) in &pairs {
        let serving = |name: &str, stage: Stage| {
            members
                .iter()
                .any(|member| member.node == name && member.serves(stage))
        };
        assert!(serving(from, *from_stage) && serving(to, *to_stage));
        assert_eq!(from_stage.next(), Some(*to_stage));
    }
    assert!(handed(&[]).is_empty());
}
