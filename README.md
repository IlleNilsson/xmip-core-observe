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
words, each value's `word` and the `name` a person reads it by, and a link's rate over two readings, `Topology::rate_since`, which the
publisher states and no surface computes). `Publication::write` writes
it whole or not at all, by `publication::write_atomic`, the one writer of
every file a surface reads a publisher's state from. The topology is drawn
here too, once for every publisher (ADR-0052, amendment 2026-09-30):
`topology::draw` draws a cluster, its nodes, their declared or reported
stages and an endpoint per segment beneath a stage, each Fine or Holding
over its worst leaf; `topology::party` adds the Party on each side a stage
faces, linked with what the stage counted. The Playground's roll and a
running node (the runtime's `Running::publication`, which `xmip-service`
writes) both draw through them. A node's throughput over time is `Curve`, the history file, and the
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
(`Snapshot::record_dead_message`), as `[[dead_messages]]`; and, for each
Send Port with Journeys that failed waiting in its queue, how many and the
oldest with why (`FailedJourneys`: node, Send Port, count, and each
`FailedJourney`'s identifier, place and reason; `runtime-model.md` section
13), kept by node and Port (`Snapshot::record_failed_journeys`), as
`[[failed_journeys]]`. `orders` is the
directory where its publisher takes an operator's act on any of them:
`Order` writes and takes the file, `Act` names pause, resume, remove,
replay, retry and dismiss once, and `Noun` says which acts each takes — an
Event subscription pause, resume and remove, a Subscription pause and resume
only, since it is added and removed in the TOML configuration, a Message in
the Dead Message Queue (`dead-message`) replay only, and a Journey that
failed (`journey`, by its identifier) retry and dismiss (`runtime-model.md`
section 13, built 2026-10-04).

A scope is an Xmip URI, and the tree is its path (ADR-0027 clauses 3 and 4).
`Scope` reads one — the scheme and the authority go, a slash at either end
is ignored, empty text is the root — `Scope::segments` splits it, and
`Scope::contains` is the one containment rule the snapshot and the activity
log answer by: at and beneath, by segment, never by character. `Scope::node`
is the node a scope is on — the segment after the `node` marker beneath the
cluster, `xmip:///<cluster>/node/<name>` (ADR-0053), and none for a scope on
no node, since the cluster is never one — and `Scope::stage` the stage of the
message path it is on, beneath its node or its cluster, so no name is read as
a stage.

The one wildcard over scopes is `wildcard::matches`: `*` and `?`, literal
everything else, case-insensitive, `*` crossing a `/`, both sides read as
scopes first (ADR-0052, amendment 2026-09-19). It moved here from
`Xmip.Surface`'s `ScopePattern` on 2026-09-29, when the audit read needed it
in Rust; the runtime forwards it as `xmip_scope_matches_v1`.

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
rather than keep a copy; the Playground writes and reads its files through
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
