# xmip-core-observe

Observation: what is happening now, and what is unhealthy. What the message
path writes lands in a `Snapshot` over the scope tree, with its activity and
history, and the operator boundary in `xmip_operate.h` reads that snapshot
and nothing else (ADR-0027 clause 6).

Observation is near-real-time and never synchronous: Receive, Process and
Send never wait for it. It is not the durable record — audit is.

Health is a mood — Fine, Paused, Working, Stressed, Exhausted, Done, with
Holding the rollup a parent shows (ADR-0041) — and `Health` is where each is
named once: its word (`word`, and `named` back from it), the name of the
color a surface paints it in (`color`; the paint stays the surface's), what
a parent shows over it (`rolled`: Fine or Holding), and the worst-first order
every reader returns records in (`Standing`: the worse mood, then the higher
severity, then the scope). `Counted` is what a count counts, with its word
and the kind each stage of the message path counts (`at`).

A publication is a snapshot as the file a surface reads: `Publication`
writes it (`whole`: every count at the scope it was recorded at, so a
reader sums what is beneath any scope — a roll's file and a node's alike) and reads
it back, with the records, the counts, what the run was started with (`Run`)
and the communication topology (`topology`: its nodes, links and their
words, each value's `word` and the `name` a person reads it by). What moved
is `topology::window::Window`'s, which the publisher keeps and no surface
computes: over its earlier readings of the last thirty seconds it states on
each drawing every link's rate and every drawn thing's activity, per
second — the most of the Streams, Journeys or Messages counted on a stage
of the message path (`node::Scope::stage`) at and beneath it, never a test's
gauge beneath a node, the cluster's beneath its nodes, or the more of what
its links carried in or out — zero where nothing moved, a figure that fell measured from its
fall, and a clock that steps back starting the window over (ADR-0052,
amendment 2026-11-10). It keeps about one reading a second in the
drawing's own order and reads a round in one pass; the module's timing
program (`tests/measured`, run alone at landing) holds a drawing of 1,201
things, 3,000 counts and 1,000 links under a millisecond. `Publication::write` writes
it whole or not at all, by `publication::write_atomic`, the one writer of
every file a surface reads a publisher's state from. A reader showing many
publishers as one merges them here once: `Snapshot::merge`, every table,
and `Topology::merge`, a node's own drawing winning for itself and what is
beneath it but for a mood the reader holds worse, and its links where
either end is its own (ADR-0052, amendments 2026-11-11); `merge_each`
merges every node's drawing through one index, and `merge` is its
one-drawing case. What no node can say of itself, the rollup at
`<cluster>/node` — the worst leaf beneath every node, one point below it —
is `rollup::nodes`, which the Essentials' cluster process and the runtime's
gatherer (`runtime::plane::gather`) call alike (ADR-0052, amendment
2026-11-13). A reader asking of many scopes at once searches a snapshot's
records ordered by path once, `snapshot::Beneath`, finding what
`Snapshot::health` finds; the drawing and the relations read through it
and write their ids and scopes in one allocation each
(`topology::draw::joined`), so a gatherer's view of a cluster drawing 176
things is assembled within a millisecond. Where the work at
a scope came from and went is said here once too, `topology::from_to`, for
every surface (`xmip_publication_from_to_v1`). The topology is drawn
here too, once for every publisher (ADR-0052, amendment 2026-09-30):
`topology::draw` draws a cluster, its nodes — each saying the roles it
declares and, a Storage node, its place in `[storage] nodes` — their
declared or reported stages and an endpoint per segment beneath a stage,
each Fine or Holding over its worst leaf and still until a window states
what moved; `topology::party` adds the Parties on each side a stage
faces — where a publication cannot say which Parties a Location takes, one
facing each node on each side, labelled *TX-Party 1, 2, …* sending in and
*RX-Party 1, …* delivered to, numbered per side in the order the nodes
stand — linked with what the stage counted. The Essentials' roll and a running node (the
runtime's `Running::publication`, which `xmip-service` writes) both draw
through them, and each states its drawing over a window of its own.

How the cluster's nodes are related is `topology::relate`, by role and the
order of `[storage] nodes` and never by a name: every node not declaring
storage *writes into* the first listed, which *hands work to* every node
serving process or send and *copies to* each later-listed one; a node
declaring operational or monitoring *operates* or *monitors* every other —
each configured until traffic crosses it (`topology::Relation`; ADR-0052
and ADR-0056, amendments 2026-11-13). Every link is named by one rule,
`topology::link_id(from, to, relation)`, so a configured relation and the
traffic observed over it merge into one line, configured and observed
(`Origin::joined`). Where each thing stands on the map is
`topology::layout`, computed by the reader (`xmip_publication_layout_v1`):
TX-Parties, receiving, Storage, sending, RX-Parties left to right, the
operation band on top, processing over Storage, standbys below the first
listed, executing in a lane of its own; never by traffic, never by a name.
Its tests hold a renamed cluster to the same bands and columns, swapped
roles to swapped places, and every line clear of the things it does not
join. A node's throughput over time is `Curve`, the history file, and the
recent items beneath a scope are `Recent`, the activity file; each is
written and read here, as a publication is. What a reader does not know it
decides here once — an unknown mood
shows as Stressed, an unknown counted kind is skipped. Where a node
publishes its capability, and how that record reads back, is `capability`,
over `xmip-core-node`'s `Capability`.

A publication also carries the Subscriptions its nodes route by
(`Subscription`: its configured name, filter, destination, the file and the
entry it is configured in, whether it is paused, what it picked up and what
it holds; ADR-0013, amendment 2026-09-30), kept by node and name
(`Snapshot::record_subscription`), and the Event subscriptions its nodes'
hubs hold (`EventSubscription`; ADR-0065, amendment 2026-09-29), kept by node
and number (`Snapshot::record_event_subscription`). Both say active or
paused in `PauseState`'s words. It carries too the oldest Messages each
node's Dead Message Queue keeps (`DeadMessage`: node, Message, place, Receive
Location, time, gate verdicts, promoted properties and every Subscription's
decline; ADR-0052, amendment 2026-10-01), kept by node and place
(`Snapshot::record_dead_message`), as `[[dead_messages]]`; and, for every
Send Port a node sends, the Messages that failed waiting in its queue now —
how many, zero where none, whether one blocks its sequence, and the oldest
with why — apart from the last that failed there since the node started,
which is history (`FailedMessages`: node, Send Port, `count`, `blocked`, each
`FailedMessage`'s identifier, place and reason, and `last_failure`;
`runtime-model.md` section 13), kept by node and Port
(`Snapshot::record_failed_messages`), as `[[failed_messages]]`. A snapshot
is read and never acted on: an operator's act on any of them is asked at a
node's door and reaches its node through Xmip Storage (ADR-0014, amendment
2026-11-13). `Order` is one act asked of one node, `Act` names pause, resume,
remove, replay, retry and dismiss once, and `Noun` says which acts each takes — an
Event subscription pause, resume and remove, a Subscription pause and resume
only, since it is added and removed in the TOML configuration, a Message in
the Dead Message Queue (`dead-message`) replay only, and a Message that
failed (`message`, by its Journey's identifier) retry and dismiss (`runtime-model.md`
section 13, built 2026-10-04).

A scope is an Xmip URI, and the tree is its path (ADR-0027 clauses 3 and 4):
the snapshot and the activity log answer at and beneath a scope by
`xmip-core-node`'s one rule, `node::Scope::contains`.

Whether something a run made is shown is `run::shown`: always where the run
declared nothing, and where `Run::hidden` says it declared itself hidden only
when the reader asks to include what is hidden. The audit capability's query
applies it to records, and the runtime forwards it as `xmip_run_shown_v1` for
every other surface (ADR-0028, amendment 2026-09-30).

A record is stamped by `observe::now_unix_nanos`: the estate's one clock,
`xcore::SystemClock`, in the unit a record and `xmip_operate.h` carry —
nanoseconds since the Unix epoch as an `i64`.

These are written once, here, and nowhere else. The runtime's cdylib
forwards each to the surfaces over `xmip_operate.h` sections 7 and 8 — a
publication and a curve are read by the runtime and handed over as the
header's values —
and `Xmip.Surface`, the PowerShell module and the GUI call those exports
rather than keep a copy; the Essentials writes and reads its files through
`Publication` (ADR-0052, amendments 2026-09-24).

## What an operator's own monitoring reads

A figure is what an exporter writes of a snapshot (`figure`): its name
segment by segment, its unit, a line saying what it is, whether it is a
level (`Kind::Gauge`) or a count over its window (`Kind::Window`), and
its points, one per scope. `FIGURES` is every one — `xmip.health` (a
mood by its rank, `Health::rank`, `fine` 0 to `holding` 6, its word
beside it), `xmip.health.severity`, and one per `Counted` kind named by
its word — and `Reading` reads a snapshot for an export in one pass: its
scopes once each, in order, and each figure's points knowing which scope
they are of (`Point::at`), so an exporter writes a scope's text once.
Each exporter is a technology mounted beside this crate's source, which
is why that source is `.src` (ADR-0049), and each writes the same figures
in its own format:

| Technology | What it does |
| --- | --- |
| [`otlp`](https://github.com/IlleNilsson/xmip-core-observe-otlp) | OpenTelemetry metrics, protobuf over OTLP/HTTP, pushed on every change from a thread of its own |
| [`prometheus`](https://github.com/IlleNilsson/xmip-core-observe-prometheus) | A scrape endpoint in the text exposition format 0.0.4, rendered at each scrape from the last snapshot |

`doc/architecture/observability-model.md` sections 6 and 7 govern it, and
`architecture.toml` names the technologies.
