# Compact indentation: production API measurements

v0.15.0 omits indentation in compact tables, shared messages and self-contained
files. The normal writer, parser, name table assignment, table ID and protocol
version remain unchanged. This reduces bytes; it does not guarantee faster
serialization or deserialization.

## Correctness

Production tests first failed in all three added cases under the indented codec.
After implementation, 13 compact tests and the corpus test pass. They check:

- Unindented table/message/file output and unchanged ordinary pretty output.
- Parent/child structure, string whitespace, multiline values, raw/escaped
  strings, annotations, duplicate properties, Unicode and special float values.
- Previously indented messages/files and table imports, with stable table IDs.
- All 233 valid official KDL test documents round-trip through both envelopes,
  and their serialized forms are accepted with matching meaning by kdl-rs.

The corpus test is excluded from packages along with the corpus; standalone
compact acceptance tests remain included. Research code is not packaged.

## Bytes: existing typed benchmark fixture

The existing `name_shortcuts_runtime` benchmark includes AST construction, name
lookup/remapping and output. Its self-contained encode also builds the table;
decode includes parsing and conversion back to the derived Rust types.

| Fixture | Ordinary KDL | Shared before → after | Self-contained before → after |
|---|---:|---:|---:|
| 1 record, 8-byte payload | 227 B | 163 → 159 B | 323 → 279 B |
| 100 records, 8-byte payload | 16,178 B | 6,016 → 5,616 B | 6,176 → 5,736 B |
| 1 record, 4 KiB payload | 4,315 B | 4,251 → 4,247 B | 4,411 → 4,367 B |

The exchanged table falls from 241 B to 201 B. All values, including the payload,
are identical in each before/after pair.

## Timing: two controls

Apple M5, macOS 27.2, rustc 1.96.0, Criterion 0.8.2, single-threaded release build,
shared machine, 2026-10-30. Wall time includes result destruction, not network IO.

The initial sequential before/after runs used the existing typed benchmark,
40 samples, 200 ms warmup and 1 second measurement. Unchanged controls drifted:
for 100 records normal encode changed from 15.92 to 17.84 µs and table construction
from 22.54 to 26.54 µs. Those changes do not isolate the indentation effect. Raw
estimates and 95% confidence intervals are retained in
[indent-pack-results.csv](indent-pack-results.csv); no speed claim is based on them.

A second comparison loads the historical compact module from main `3f10721` and
the current module into **one temporary crate copy**. Both use the same current
AST/derive/parser/value writer. The historical module retains its original
indented output calls. Nothing from this harness is inserted into the production
checkout. Assertions verify that both old and new decoders accept both outputs
and recover the same complete typed object.

This fixture has 100 records with three properties each: identifier, position
and visibility. It is different from the six-property fixture above. Shared
bytes fall from 3,432 to 3,032 B; self-contained bytes from 3,494 to 3,070 B.
The run uses 60 samples, 300 ms warmup, 2 seconds measurement, 10,000 resamples.

| Typed operation | Before | After |
|---|---:|---:|
| Shared encode | 12.983 µs | 12.532 µs |
| Shared decode | 19.378 µs | 19.406 µs |
| Self-contained encode | 21.883 µs | 23.153 µs |
| Self-contained decode | 21.249 µs | 20.987 µs |

[Paired estimates with 95% intervals](indent-pack-paired-results.csv).
This shallow workload does **not** show a consistent end-to-end speedup. Shared
encode improves slightly, self-contained encode is slower, and reads are close.
The result supports releasing a size optimization, not advertising a universal
speed improvement. Deep-tree parser gains from the
[earlier exploration](../../research/indent-pack/README.md) remain separate
microbenchmarks and must not be applied to this typed API table.

Both comparisons use fixed ordering and a single run. CPU time, p99, RSS,
compression and application/network latency were not measured. Representative
downstream application data remains a separate follow-up.

## Reproduce

From the repository root, run the actual current API benchmark:

```sh
cargo bench --bench name_shortcuts -- name_shortcuts_runtime --sample-size 40 --warm-up-time 0.2 --measurement-time 1 --noplot
```

For the same-process historical/current comparison, generate the historical
source and a local dependency lock first (the root library does not track a lock):

```sh
cargo generate-lockfile
python3 research/indent-pack/generate.py
python3 research/indent-pack/run-codec.py
```

The harness uses a temporary copy, preserves the root checkout, and writes
Criterion results under `research/indent-pack/target/paired-codec-results`.
Inputs and timing definitions are in `research/indent-pack/codec_comparison.rs`.
Historical source generation requires commit `3f10721` to exist locally.
