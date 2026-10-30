# Rule-based automatic aliases: runtime measurements

The implemented codec reduces bytes but **does not improve CPU time** in these
cases. Its runtime tree construction and dictionary lookups cost more than the
unchanged normal APIs. The [earlier fixed-name experiment](name-shortcuts.md)
measures a different, compile-time mapping path; its speedup does not apply here.

Measured 2026-10-30 on Apple M5, macOS 27.2 (26B123), rustc 1.96.0, optimized
Cargo bench profile, Criterion 0.8.2. Each of the 21 measurements uses 60 samples,
500 ms warm-up and 2 s measurement time. This is a single local run on a shared
development machine, with normal APIs remeasured in that same run. No other
test/build/benchmark job was launched concurrently by this session.

The fixtures are the same as the earlier experiment: one record with an 8-byte
value, 100 records with 8-byte values, and one record with a 4096-byte value of
repeated `x`. Values are synthetic, not a production traffic sample. Times
include output destruction and are per complete message, not per byte.

- `normal`: ordinary `to_string/from_str`, readable original names.
- `shared`: `NameTable::encode/decode`, table built outside timing, but includes
  the message header, tree construction, lookup, parsing and name expansion.
- `file`: `compact::to_string/from_str`, including table generation/import,
  validation and hashing, and embedded table writing/parsing each time.

All formats restore the same original Rust type and are accepted by `kdl-rs`.
The [raw results](name-shortcuts-runtime-results.csv) retain the estimator and
95% confidence intervals. Estimates are Criterion regression slopes or a mean
fallback for flat sampling. Relative changes below use point estimates, not
paired statistical tests. Negative byte savings mean the output grew.

| Case | Mode | Wire/file bytes | Bytes saved | Serialize µs | Deserialize µs |
|---|---|---:|---:|---:|---:|
| small | normal | 227 | 0.0% | 0.196 | 0.513 |
| small | shared | 163 | 28.2% | 0.575 | 0.695 |
| small | file | 323 | -42.3% | 3.548 | 3.379 |
| batch100 | normal | 16,178 | 0.0% | 15.346 | 33.055 |
| batch100 | shared | 6,016 | 62.8% | 25.363 | 36.872 |
| batch100 | file | 6,176 | 61.8% | 41.509 | 39.541 |
| payload4k | normal | 4,315 | 0.0% | 1.334 | 2.131 |
| payload4k | shared | 4,251 | 1.5% | 1.803 | 2.535 |
| payload4k | file | 4,411 | -2.2% | 4.734 | 4.933 |

Table construction: small: 2.579 µs, batch100: 22.590 µs, payload4k: 2.575 µs.

The actual table is 241 bytes. Shared messages include an 81-byte header in this
fixture; table exchange is additional and amortized by reuse. The file embeds
the table without a redundant message hash/header. This differs from the
illustrative 270-byte table in the fixed-name experiment.

The small shared message costs about 2.9× serialization time and 1.4×
deserialization time compared with normal APIs. The 100-record case costs about
1.7× and 1.1×. Self-contained tables add further overhead. The shared codec
builds and remaps an AST; the normal derived writer emits text directly.

100-record storage still shrinks from 16,178 to 6,176 bytes (61.8%). The small
self-contained file grows from 227 to 323 bytes; a file dominated by a 4KB value
also grows. Compact storage suits repeated names; human-readable configuration
uses normal APIs. Measure the CPU/byte tradeoff in the intended application.
A future direct writer/parser integration could reduce overhead, but is not
implemented in this version.

Transport negotiation, packet framing beyond these KDL envelopes, compressed
traffic and real network latency/throughput are outside this measurement.

## Reproduce

```sh
mkdir -p target/name-shortcuts
KDL_SHORTCUT_ARTIFACTS=target/name-shortcuts/inputs \
  cargo bench --bench name_shortcuts -- name_shortcuts_runtime --noplot \
  > target/name-shortcuts/runtime.log 2>&1
```

To execute assertions and smoke each path without timing, run
`cargo bench --bench name_shortcuts -- --test`.
Inputs/logs are under `target/name-shortcuts`; Criterion samples and estimates
are under `target/criterion/name_shortcuts_runtime_*`.

The implementation and format are described in [design 01](../design/01-name-shortcuts.md)
and the [usage guide](../guide/01-compact-names.md).
