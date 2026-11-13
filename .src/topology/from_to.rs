//! From and to of a scope: where what is observed at it came from and
//! where it went (the owner, 2026-11-10: *from and to in every
//! observation*; ADR-0052, amendment 2026-11-11). Said here once, and read
//! by every surface through `xmip_operate.h` section 8
//! (`xmip_publication_from_to_v1`).
//!
//! The hops follow the integration's one forward path as the drawing and the
//! Subscriptions say it (`terminology.md`, Integration): the sending Party
//! and the transport it sends over, the Receive Location and its node, each
//! Subscription that routed what the Location took, the Storage node it was
//! written into, each Send Port those Subscriptions reach and its node, and
//! the Party it delivers to over its transport. What the drawing has not
//! seen yet — no Party link, no Location feeding a Subscription, no Storage
//! traffic — is left out, never guessed. A Party facing one node is said as
//! it is drawn, `TX-Party 1` or `RX-Party 2`, until the configuration names
//! which a Location takes (open problem 33).
//!
//! A Receive Location's, a Subscription's and a Send Port's scope, and what
//! is beneath one, give that path; a node's gives the Parties and nodes
//! that hand it work and those it hands work to. A Dead Message Queue entry
//! comes from its Receive Location and goes nowhere: its to is [`UNROUTED`].

use std::collections::BTreeSet;

use node::Stage;
use serde::Serialize;

use super::{Topology, TopologyLink, XMIP_STORAGE};
use crate::scope::{NODE, Scope};
use crate::snapshot::Snapshot;

/// Where what a Subscription takes none of goes: what a Dead Message Queue
/// entry's to says.
pub const UNROUTED: &str = "no Subscription";

/// One step of the path: what it is, by the words a person reads it by, its
/// name, the node it runs on — none for a Party — and the transport it is
/// reached over, where the drawing says.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Hop {
    pub kind: &'static str,
    pub name: String,
    pub node: String,
    pub transport: String,
}

impl Hop {
    /// The hop as a person reads it in a from or a to: `Sales orders to ERP
    /// [TX2]`, `TX-Party 1 over file`.
    #[must_use]
    pub fn words(&self) -> String {
        match (self.node.is_empty(), self.transport.is_empty()) {
            (false, _) => format!("{} [{}]", self.name, self.node),
            (true, false) => format!("{} over {}", self.name, self.transport),
            (true, true) => self.name.clone(),
        }
    }
}

/// A scope's from and to, in words, and the hops between them in path
/// order; all empty where the scope is on no node.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct FromTo {
    pub from: String,
    pub to: String,
    pub hops: Vec<Hop>,
    /// What a Dead Message Queue entry's to says ([`UNROUTED`]).
    pub unrouted: &'static str,
    /// The one line every surface says it in, `from <from> → to <to>`;
    /// empty where nothing is said.
    pub line: String,
    /// The whole path, each hop's words in order, joined by `→`.
    pub path: String,
}

/// The from and to of `scope`, read from `snapshot` and, where the
/// publisher drew one, `topology`.
#[must_use]
pub fn from_to(snapshot: &Snapshot, topology: Option<&Topology>, scope: &str) -> FromTo {
    let empty = Topology::default();
    let path = Path {
        snapshot,
        topology: topology.unwrap_or(&empty),
        root: root(scope),
    };
    let segments: Vec<&str> = Scope::new(scope).segments().collect();
    let hops = match segments.as_slice() {
        [_, NODE, node, stage, name, ..] => match Stage::named(stage) {
            Some(Stage::Receive) => path.received(node, name),
            Some(Stage::Process) => path.routed(node, name),
            Some(Stage::Send) => path.sent(node, name),
            None => Vec::new(),
        },
        [_, NODE, node] => path.around(node),
        _ => Vec::new(),
    };
    said(hops)
}

/// The hops, with the first as the from and the last of the last kind,
/// reached the same way — over a transport or not — as the to: a node's
/// own hop is never its to.
fn said(hops: Vec<Hop>) -> FromTo {
    let from = hops.first().map(Hop::words).unwrap_or_default();
    let last = hops.last().map(|hop| (hop.kind, hop.transport.is_empty()));
    let to: Vec<String> = hops
        .iter()
        .rev()
        .take_while(|hop| Some((hop.kind, hop.transport.is_empty())) == last)
        .map(Hop::words)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    let to = to.join(", ");
    let line = if hops.is_empty() {
        String::new()
    } else {
        format!("from {from} → to {to}")
    };
    let path = hops.iter().map(Hop::words).collect::<Vec<_>>().join(" → ");
    FromTo {
        from,
        to,
        hops,
        unrouted: UNROUTED,
        line,
        path,
    }
}

/// `xmip:///<cluster>`: what a scope beneath a node is under.
fn root(scope: &str) -> String {
    let cluster = Scope::new(scope).segments().next().unwrap_or_default();
    format!("xmip:///{cluster}")
}

/// What the path is read from.
struct Path<'a> {
    snapshot: &'a Snapshot,
    topology: &'a Topology,
    root: String,
}

impl Path<'_> {
    /// The links the drawing holds into `to`.
    fn links_into<'b>(&'b self, to: &'b str) -> impl Iterator<Item = &'b TopologyLink> {
        self.topology.links.iter().filter(move |link| link.to == to)
    }

    /// The links the drawing holds out of `from`.
    fn links_out_of<'b>(&'b self, from: &'b str) -> impl Iterator<Item = &'b TopologyLink> {
        self.topology
            .links
            .iter()
            .filter(move |link| link.from == from)
    }

    /// What a Receive Location `name` on `node` took, from its Party to
    /// where its Subscriptions sent it.
    fn received(&self, node: &str, name: &str) -> Vec<Hop> {
        let location = format!("node/{node}/receive/{name}");
        let routed: Vec<String> = self
            .links_out_of(&location)
            .filter_map(|link| link.to.rsplit('/').next().map(str::to_string))
            .collect();
        let mut hops = self.sending(node);
        hops.push(hop("Receive Location", name, node));
        hops.extend(routed.iter().map(|one| hop("Subscription", one, node)));
        hops.extend(self.onward(node, &routed));
        hops
    }

    /// What the Subscription `name` on `node` routed, from the Locations
    /// that fed it to where it sent it.
    fn routed(&self, node: &str, name: &str) -> Vec<Hop> {
        let subscription = format!("node/{node}/process/{name}");
        let mut hops = self.sending(node);
        hops.extend(self.links_into(&subscription).filter_map(|link| {
            let location = link.from.strip_prefix(&format!("node/{node}/receive/"))?;
            Some(hop("Receive Location", location, node))
        }));
        hops.push(hop("Subscription", name, node));
        hops.extend(self.onward(node, &[name.to_string()]));
        hops
    }

    /// What the Send Port `name` on `node` sent: from the Subscriptions,
    /// on whichever node, whose destination reaches it, to its Party.
    fn sent(&self, node: &str, name: &str) -> Vec<Hop> {
        let reaching: Vec<(&str, &str)> = self
            .snapshot
            .subscriptions()
            .filter(|one| one.send_ports.iter().any(|port| port == name))
            .filter_map(|one| Some((Scope::new(&one.node).node()?, one.name.as_str())))
            .collect();
        let mut hops = Vec::new();
        for (on, subscription) in &reaching {
            hops.extend(
                self.routed(on, subscription)
                    .into_iter()
                    .take_while(|hop| !(hop.kind == "Subscription" && hop.name == *subscription)),
            );
            hops.push(hop("Subscription", subscription, on));
        }
        let routing: BTreeSet<&str> = reaching.iter().map(|(on, _)| *on).collect();
        hops.extend(routing.iter().flat_map(|on| self.stored(on)));
        hops.push(hop("Send Port", name, node));
        hops.extend(self.receiving(node));
        dedup(hops)
    }

    /// The Parties and nodes that hand `node` work, the node, and those it
    /// hands work to.
    fn around(&self, node: &str) -> Vec<Hop> {
        let own = format!("node/{node}");
        let mine = |id: &str| {
            id.strip_prefix(&own)
                .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
        };
        let mut hops: Vec<Hop> = self
            .topology
            .links
            .iter()
            .filter(|link| mine(&link.to) && !mine(&link.from))
            .map(|link| self.end(&link.from, link))
            .collect();
        hops.push(hop("Node", node, node));
        hops.extend(
            self.topology
                .links
                .iter()
                .filter(|link| mine(&link.from) && !mine(&link.to))
                .map(|link| self.end(&link.to, link)),
        );
        dedup(hops)
    }

    /// The far end `id` of `link` as a hop: a Party, by what it is drawn
    /// as — `TX-Party 2` facing one node — or a node.
    fn end(&self, id: &str, link: &TopologyLink) -> Hop {
        let mut segments = id.split('/');
        match (segments.next(), segments.next(), segments.next()) {
            (Some("party"), Some(side), Some(party)) => Hop {
                kind: if side == "receiving" {
                    "Receiving Party"
                } else {
                    "Sending Party"
                },
                name: self
                    .topology
                    .nodes
                    .iter()
                    .find(|drawn| drawn.id == id)
                    .map_or_else(|| party.to_string(), |drawn| drawn.label.clone()),
                node: String::new(),
                transport: link.protocol.clone(),
            },
            (Some(NODE), Some(other), _) => Hop {
                transport: link.protocol.clone(),
                ..hop("Node", other, other)
            },
            _ => Hop {
                kind: "Party",
                name: id.to_string(),
                ..Hop::default()
            },
        }
    }

    /// The Party that sends into `node`'s receive stage, over what.
    fn sending(&self, node: &str) -> Vec<Hop> {
        let stage = format!("node/{node}/receive");
        self.links_into(&stage)
            .filter(|link| link.from.starts_with("party/"))
            .map(|link| self.end(&link.from, link))
            .collect()
    }

    /// The Party `node`'s send stage delivers to, over what.
    fn receiving(&self, node: &str) -> Vec<Hop> {
        let stage = format!("node/{node}/send");
        self.links_out_of(&stage)
            .filter(|link| link.to.starts_with("party/"))
            .map(|link| self.end(&link.to, link))
            .collect()
    }

    /// The Storage node `node` writes its Publications into, where the
    /// drawing measured it.
    fn stored(&self, node: &str) -> Vec<Hop> {
        let own = format!("node/{node}");
        self.links_out_of(&own)
            .filter(|link| link.protocol == XMIP_STORAGE)
            .filter_map(|link| link.to.strip_prefix("node/"))
            .map(|storage| hop("Storage node", storage, storage))
            .collect()
    }

    /// Where the Subscriptions `routed` on `node` sent what they took: the
    /// Storage node, each Send Port they reach on the node that sends it,
    /// and the Party each delivers to.
    fn onward(&self, node: &str, routed: &[String]) -> Vec<Hop> {
        let ports: BTreeSet<&str> = self
            .snapshot
            .subscriptions()
            .filter(|one| Scope::new(&one.node).node() == Some(node))
            .filter(|one| routed.contains(&one.name))
            .flat_map(|one| one.send_ports.iter().map(String::as_str))
            .collect();
        if ports.is_empty() {
            return Vec::new();
        }
        let mut hops = self.stored(node);
        let mut sending = Vec::new();
        for port in ports {
            let on = self.sending_node(port);
            hops.push(hop("Send Port", port, on.unwrap_or_default()));
            sending.extend(on);
        }
        let sending: BTreeSet<&str> = sending.into_iter().collect();
        hops.extend(sending.into_iter().flat_map(|on| self.receiving(on)));
        dedup(hops)
    }

    /// The node that sends the Send Port `port`: the one that published it.
    fn sending_node(&self, port: &str) -> Option<&str> {
        self.snapshot.health_records().find_map(|record| {
            let scope = Scope::new(&record.scope);
            let mut beneath = scope.segments().skip(3);
            let at = (beneath.next(), beneath.next(), beneath.next());
            match at {
                (Some(stage), Some(name), None)
                    if stage == Stage::Send.name()
                        && name == port
                        && record.scope.starts_with(&self.root) =>
                {
                    scope.node()
                }
                _ => None,
            }
        })
    }
}

/// A hop of `kind` called `name` on `node`.
fn hop(kind: &'static str, name: &str, node: &str) -> Hop {
    Hop {
        kind,
        name: name.to_string(),
        node: node.to_string(),
        transport: String::new(),
    }
}

/// `hops` with each hop once, in the order first met.
fn dedup(hops: Vec<Hop>) -> Vec<Hop> {
    let mut kept: Vec<Hop> = Vec::new();
    for one in hops {
        if !kept.contains(&one) {
            kept.push(one);
        }
    }
    kept
}

#[cfg(test)]
mod tests;
