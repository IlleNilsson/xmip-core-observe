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
writes it (`of` for a roll's sums, `whole` for a node's own file) and reads
it back, with the records, the counts, what the run was started with (`Run`)
and the communication topology (`topology`: its nodes, links and their
words). A node's throughput over time is `Curve`, the history file, and the
recent items beneath a scope are `Recent`, the activity file; each is
written and read here, as a publication is. What a reader does not know it
decides here once — an unknown mood
shows as Stressed, an unknown counted kind is skipped. Where a node
publishes its capability, and how that record reads back, is `capability`,
over `xmip-core-node`'s `Capability`.

A scope is an Xmip URI, and the tree is its path (ADR-0027 clauses 3 and 4).
`Scope` reads one — the scheme and the authority go, a slash at either end
is ignored, empty text is the root — `Scope::segments` splits it, and
`Scope::contains` is the one containment rule the snapshot and the activity
log answer by: at and beneath, by segment, never by character.

These are written once, here, and nowhere else. The runtime's cdylib
forwards each to the surfaces over `xmip_operate.h` sections 7 and 8 — a
publication and a curve are read by the runtime and handed over as the
header's values —
and `Xmip.Surface`, the PowerShell module and the GUI call those exports
rather than keep a copy; the Playground writes and reads its files through
`Publication` (ADR-0052, amendments 2026-09-24).

`doc/architecture/observability-model.md` sections 6 and 7 govern it; each
exporter is a technology under this repository, and `architecture.toml` names
them.
