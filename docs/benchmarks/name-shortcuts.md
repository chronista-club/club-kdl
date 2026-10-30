# KDL name shortcuts: fixed-schema experiment

This is the initial, pre-implementation experiment. The subsequent
[runtime codec measurements](name-shortcuts-runtime.md) include the implemented
dictionary handling and framing; they show byte savings with increased CPU time.
The implementation implications at the end record the design discussion at
the time of this initial experiment; see [design 01](../design/01-name-shortcuts.md)
for the implemented protocol.

Date: 2026-10-30. Source baseline: `8938d5b`, club-kdl 0.14.1.
Local experiment branch: `spike/name-shortcuts`.

User constraint: 「そうね。KDL自体のspecは準拠で。」

## Question and scope

Does replacing long node/property names with short wire names reduce message
size and serialization/deserialization time, while retaining standard KDL v2?

This benchmark uses existing derive `name` and `property(rename = ...)`
attributes. There are no production-library changes. Both ends know the mapping
at compile time. It measures the potential benefit before implementing automatic
alias allocation, negotiation or runtime dictionary lookups.

Four encodings use identical Rust fields, values, ordering and nesting:

- `normal`: ten readable node/property names, 12–19 ASCII bytes each.
- `short1`: `a` through `j`.
- `short2`: `aa` through `aj`, representing a namespace whose one-character names
  have been exhausted. This is a fixed mapping, not an allocation algorithm.
- `numeric`: string names `"0"` through `"9"`. Quotes count toward the size.
  These are KDL string identifiers, not integer identifiers or binary indices.

Under the [KDL v2 specification](https://kdl.dev/spec/), node names and property
keys are strings. Short names and quoted numeric names need no syntax extension.
Mapping names back to application fields is an application-protocol concern.

## Method

Environment: Apple M5, macOS 27.2 (26B123), rustc 1.96.0
(`ac68faa20`, 2026-05-25), Criterion 0.8.2, optimized Cargo bench profile.
Each benchmark uses 60 samples, 500 ms warm-up and 2 s measurement time.
One operation is one complete message; timings include output destruction.
This is one local run on a shared development machine, not a dedicated host.

Fixtures:

| Case | Records per message | Payload per record |
|---|---:|---:|
| small | 1 | 8 ASCII bytes |
| batch100 | 100 | 8 ASCII bytes |
| payload4k | 1 | 4096 ASCII bytes |

The large payload is repeated `x` characters: a size-dominance control, not
representative application content or a compression benchmark. Values and all
whitespace are identical between formats. The standard writer's indentation
is included. There is no gzip/zstd or transport framing in the size totals.

Timed operations:

- `serialize_fresh`: `club_kdl::to_string`, with a fresh output allocation.
- `serialize_reuse`: `to_string_into` with a reused buffer, preallocated to the
  normal message size for every format. This controls for allocation/growth.
- `deserialize`: `club_kdl::from_str` including parsing and owned object creation.

Before timing each case, assertions check identical logical values via JSON,
exact KDL round-trip equality, and independent `kdl-rs` parsing followed by
typed deserialization and equality. Dictionary and combined-envelope syntax
are also checked with `kdl-rs` (6.7.1). This validates these fixtures; it is not
a claim that all possible alias protocols have been tested.

An illustrative KDL dictionary (`dictionary version=1 { ... }`) includes all ten
short-to-long mappings. Its measured size is reported separately. It is not
decoded/resolved in the timed path. The number of messages required to amortize
one shared dictionary is `floor(dictionary_bytes / saved_body_bytes) + 1`.
This assumes the same schema and message size each time, with no extra handshake,
schema identifier, retry or framing bytes.

## Reproduce

```sh
mkdir -p target/name-shortcuts
KDL_SHORTCUT_ARTIFACTS=target/name-shortcuts/inputs \
  cargo bench --bench name_shortcuts -- --test
KDL_SHORTCUT_ARTIFACTS=target/name-shortcuts/inputs \
  cargo bench --bench name_shortcuts -- --noplot \
  > target/name-shortcuts/run.log 2>&1
```

The first command runs all correctness assertions and exercises each operation
once. The second records timings. Inputs and dictionaries are exported as KDL;
Criterion keeps estimates and samples under `target/criterion`.

Validation on the initial experiment's benchmark source: `cargo fmt --all --check`,
`git diff --check`, `cargo clippy --bench name_shortcuts -- -D warnings`, and
`cargo bench --bench name_shortcuts -- --test` succeeded (36 smoke operations).
The timed run completed all 36 measurements. The subsequent lint-only change
replaced the untimed dictionary division guard with `checked_div`; timed paths
and fixture data are unchanged. No production code or public API was modified.

## Results

Times below are microseconds per whole message (lower is better). Estimates
are Criterion regression slopes, with a mean fallback for flat sampling;
[raw results](name-shortcuts-results.csv) retain the estimator and 95% confidence
intervals for all 36 measurements. Percentages below use point estimates and
are descriptive comparisons, not paired statistical tests.

| Case | Format | Body bytes | Bytes saved | Serialize fresh µs | Serialize reuse µs | Deserialize µs |
|---|---|---:|---:|---:|---:|---:|
| small | normal | 227 | 0.0% | 0.202 | 0.189 | 0.524 |
| small | short1 | 82 | 63.9% | 0.143 | 0.131 | 0.448 |
| small | short2 | 92 | 59.5% | 0.156 | 0.141 | 0.462 |
| small | numeric | 102 | 55.1% | 0.143 | 0.133 | 0.469 |
| batch100 | normal | 16,178 | 0.0% | 15.863 | 15.197 | 33.837 |
| batch100 | short1 | 5,935 | 63.3% | 11.837 | 11.094 | 28.147 |
| batch100 | short2 | 6,638 | 59.0% | 11.763 | 11.440 | 29.271 |
| batch100 | numeric | 7,341 | 54.6% | 10.935 | 11.504 | 29.241 |
| payload4k | normal | 4,315 | 0.0% | 1.344 | 1.205 | 2.072 |
| payload4k | short1 | 4,170 | 3.4% | 1.293 | 1.149 | 2.021 |
| payload4k | short2 | 4,180 | 3.1% | 1.297 | 1.154 | 1.975 |
| payload4k | numeric | 4,190 | 2.9% | 1.289 | 1.151 | 2.078 |

For one-character names, fresh serialization takes 29.3% less time in `small`
and 25.4% less in `batch100`; deserialization takes 14.5% and 16.8% less,
respectively. Buffer reuse preserves the improvement (30.8% / 27.0% less
serialization time). In `payload4k`, byte savings are only 3.4%; time differences
are small and should not be generalized into a meaningful speed guarantee.
Numeric names are sometimes as fast or faster to write here despite being
larger. Shorter encoding is not a guarantee of lower CPU cost.

### Including the dictionary

| Case | Format | Dictionary bytes | Body + dictionary bytes | Messages to amortize one dictionary |
|---|---|---:|---:|---:|
| small | short1 | 270 | 352 | 2 |
| small | short2 | 280 | 372 | 3 |
| small | numeric | 270 | 372 | 3 |
| batch100 | short1 | 270 | 6,205 | 1 |
| batch100 | short2 | 280 | 6,918 | 1 |
| batch100 | numeric | 270 | 7,611 | 1 |
| payload4k | short1 | 270 | 4,440 | 2 |
| payload4k | short2 | 280 | 4,460 | 3 |
| payload4k | numeric | 270 | 4,460 | 3 |

The one-character dictionary is 270 bytes. Sending it on every small message
increases the total from 227 to 352 bytes. Sharing it once becomes smaller in
aggregate from the second small message. A 100-record file is already smaller
with its own dictionary: 16,178 → 6,205 bytes (61.6% reduction).

## Implementation implications

The user clarified the intended usage split:

> あとは、保存する時に、ファイル容量削減で、保存するファイルとかにも適用できるね。
> 人が見る設定ファイルとかは、エイリアスは使用しないで、本名の方だけ使って、今まで通り保存してつかいつつ。

Keep human-facing configuration files and the default serialization behavior
using the original names. Consider an explicit compact mode for messaging and
machine-oriented storage of the same logical objects. This is an agreed usage
direction, not an implemented API. Both representations must remain valid KDL.

For independently readable storage, an embedded dictionary is a useful starting
proposal: opening a file later must not depend on a lost connection/session
dictionary. The dictionary-inclusive size above models that cost for this
fixture. Actual file-container loading and dictionary resolution are not yet
implemented. A schema reference could be smaller but would require a durable,
versioned registry; that tradeoff remains open.

A shared mapping is necessary to interpret the short names. A future protocol
must identify the mapping version, reject unknown/mismatched mappings, define
collision handling and scope, and avoid assigning different names from message
encounter order on each peer. Evaluate savings in UTF-8 bytes, including quotes
and dictionary overhead, rather than using character count alone.

Dynamic alias assignment, expansion back to long names, dictionary transmission
latency, compressed traffic, unknown fields, schema evolution and real network
throughput remain unmeasured. A runtime adapter can consume the CPU savings
observed here; direct generated access to short names avoids that extra step.
