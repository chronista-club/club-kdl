//! Fixed, pre-shared wire names; no dynamic dictionary lookup in the timed path.
//! Run: cargo bench --bench name_shortcuts -- --noplot
//! Set KDL_SHORTCUT_ARTIFACTS to export the inputs and dictionary documents.

use club_kdl::{KdlDeserialize, KdlSerialize};
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use serde::Serialize;
use std::{fmt::Debug, hint::black_box, time::Duration};

const LONG_NAMES: [&str; 10] = [
    "object_message",
    "sequence_number",
    "sender_identifier",
    "record_update",
    "object_identifier",
    "position_horizontal",
    "position_vertical",
    "rotation_degrees",
    "display_name",
    "content_payload",
];

// Identical Rust fields, values, order and nesting; only wire names vary.
// Forward names as token trees so derive receives bare string literals,
// rather than opaque macro expression groups.
macro_rules! wire_format {
    ($module:ident, $message:tt, $seq:tt, $sender:tt,
     $record:tt, $id:tt, $x:tt, $y:tt,
     $rotation:tt, $name:tt, $payload:tt) => {
        mod $module {
            use super::*;
            pub const NAMES: [&str; 10] = [
                $message, $seq, $sender, $record, $id, $x, $y, $rotation, $name, $payload,
            ];

            #[derive(Debug, PartialEq, Serialize, KdlSerialize, KdlDeserialize)]
            #[kdl(name = $message)]
            pub struct Message {
                #[kdl(property(rename = $seq))]
                sequence_number: u64,
                #[kdl(property(rename = $sender))]
                sender_identifier: String,
                #[kdl(children, name = $record)]
                records: Vec<Record>,
            }

            #[derive(Debug, PartialEq, Serialize, KdlSerialize, KdlDeserialize)]
            #[kdl(name = $record)]
            struct Record {
                #[kdl(property(rename = $id))]
                object_identifier: u64,
                #[kdl(property(rename = $x))]
                position_horizontal: f64,
                #[kdl(property(rename = $y))]
                position_vertical: f64,
                #[kdl(property(rename = $rotation))]
                rotation_degrees: f64,
                #[kdl(property(rename = $name))]
                display_name: String,
                #[kdl(property(rename = $payload))]
                content_payload: String,
            }

            pub fn fixture(count: usize, payload_bytes: usize) -> Message {
                Message {
                    sequence_number: 42,
                    sender_identifier: "client-01".into(),
                    records: (0..count)
                        .map(|i| Record {
                            object_identifier: i as u64 + 1000,
                            position_horizontal: 12.5 + i as f64,
                            position_vertical: -3.25,
                            rotation_degrees: 90.0,
                            display_name: format!("object-{i:03}"),
                            content_payload: "x".repeat(payload_bytes),
                        })
                        .collect(),
                }
            }
        }
    };
}

wire_format!(
    normal,
    "object_message",
    "sequence_number",
    "sender_identifier",
    "record_update",
    "object_identifier",
    "position_horizontal",
    "position_vertical",
    "rotation_degrees",
    "display_name",
    "content_payload"
);
wire_format!(short1, "a", "b", "c", "d", "e", "f", "g", "h", "i", "j");
wire_format!(
    short2, "aa", "ab", "ac", "ad", "ae", "af", "ag", "ah", "ai", "aj"
);
wire_format!(numeric, "0", "1", "2", "3", "4", "5", "6", "7", "8", "9");

fn dictionary(names: &[&str; 10]) -> String {
    if names == &LONG_NAMES {
        return String::new();
    }
    // An illustrative, valid KDL protocol envelope. It is sent once per
    // schema/session, or prefixed to every message in the self-contained case.
    let mut out = String::from("dictionary version=1 {\n");
    for (alias, original) in names.iter().zip(LONG_NAMES) {
        out.push_str(&format!("  {alias:?} {original:?}\n"));
    }
    out.push_str("}\n");
    let doc = club_kdl::Document::parse(&out).unwrap();
    assert_eq!(doc.nodes()[0].children().len(), LONG_NAMES.len());
    let _: club_kdl::kdl::KdlDocument = out.parse().unwrap();
    out
}

fn artifact(name: &str, text: &str) {
    if let Some(dir) = std::env::var_os("KDL_SHORTCUT_ARTIFACTS") {
        let dir = std::path::PathBuf::from(dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(name), text).unwrap();
    }
}

fn measure<T>(
    c: &mut Criterion,
    case: &str,
    label: &str,
    value: &T,
    canonical: &serde_json::Value,
    names: &[&str; 10],
    normal_bytes: usize,
) where
    T: KdlSerialize + for<'de> KdlDeserialize<'de> + Serialize + PartialEq + Debug,
{
    let text = club_kdl::to_string(value).unwrap();
    // Correctness is outside timing: same logical object in every wire format,
    // round-trip equality, and acceptance by the independent kdl-rs parser.
    assert_eq!(&serde_json::to_value(value).unwrap(), canonical);
    let decoded: T = club_kdl::from_str(&text).unwrap();
    assert_eq!(&decoded, value);
    let external: club_kdl::kdl::KdlDocument = text.parse().unwrap();
    let decoded_external: T = club_kdl::from_kdl_document(&external).unwrap();
    assert_eq!(&decoded_external, value);

    let dict = dictionary(names);
    if !dict.is_empty() {
        let envelope = format!("{dict}{text}");
        let _: club_kdl::kdl::KdlDocument = envelope.parse().unwrap();
    }
    let saved = normal_bytes - text.len();
    // Strictly smaller total bytes: N * savings > dictionary bytes.
    let break_even = dict.len().checked_div(saved).map_or(0, |n| n + 1);
    println!(
        "SIZE,{case},{label},{},{},{},{break_even}",
        text.len(),
        dict.len(),
        text.len() + dict.len()
    );
    artifact(&format!("{case}-{label}.kdl"), &text);
    artifact(&format!("dictionary-{label}.kdl"), &dict);

    let mut group = c.benchmark_group(format!("name_shortcuts/{case}"));
    // One operation is one whole message, not a byte: shorter input must not
    // look slower just because it processes fewer bytes.
    group.throughput(Throughput::Elements(1));
    group.bench_function(BenchmarkId::new("serialize_fresh", label), |b| {
        b.iter(|| black_box(club_kdl::to_string(black_box(value)).unwrap()));
    });
    // Control for capacity growth / allocation changes around the 256 B
    // initial capacity used by to_string(). All variants get the same capacity.
    let mut buffer = String::with_capacity(normal_bytes);
    group.bench_function(BenchmarkId::new("serialize_reuse", label), |b| {
        b.iter(|| {
            buffer.clear();
            club_kdl::to_string_into(black_box(value), &mut buffer).unwrap();
            black_box(&buffer);
        });
    });
    group.bench_function(BenchmarkId::new("deserialize", label), |b| {
        b.iter(|| black_box(club_kdl::from_str::<T>(black_box(&text)).unwrap()));
    });
    group.finish();
}

fn bench_shortcuts(c: &mut Criterion) {
    println!("SIZE,case,format,body_bytes,dictionary_bytes,body_plus_dictionary,amortize_messages");
    for (case, count, payload_bytes) in [
        ("small", 1, 8),
        ("batch100", 100, 8),
        ("payload4k", 1, 4096),
    ] {
        let value = normal::fixture(count, payload_bytes);
        let canonical = serde_json::to_value(&value).unwrap();
        let normal_bytes = club_kdl::to_string(&value).unwrap().len();
        measure(
            c,
            case,
            "normal",
            &value,
            &canonical,
            &normal::NAMES,
            normal_bytes,
        );
        measure(
            c,
            case,
            "short1",
            &short1::fixture(count, payload_bytes),
            &canonical,
            &short1::NAMES,
            normal_bytes,
        );
        measure(
            c,
            case,
            "short2",
            &short2::fixture(count, payload_bytes),
            &canonical,
            &short2::NAMES,
            normal_bytes,
        );
        measure(
            c,
            case,
            "numeric",
            &numeric::fixture(count, payload_bytes),
            &canonical,
            &numeric::NAMES,
            normal_bytes,
        );
    }
}

fn bench_runtime(c: &mut Criterion) {
    use club_kdl::compact::{self, NameTable};

    println!("RUNTIME_SIZE,case,normal_bytes,shared_message_bytes,file_bytes,table_bytes");
    for (case, count, payload_bytes) in [
        ("small", 1, 8),
        ("batch100", 100, 8),
        ("payload4k", 1, 4096),
    ] {
        let value = normal::fixture(count, payload_bytes);
        let normal_text = club_kdl::to_string(&value).unwrap();
        let table = NameTable::from_value(&value).unwrap();
        let shared = table.encode(&value).unwrap();
        let file = compact::to_string(&value).unwrap();
        assert_eq!(table.decode::<normal::Message>(&shared).unwrap(), value);
        assert_eq!(compact::from_str::<normal::Message>(&file).unwrap(), value);
        let _: club_kdl::kdl::KdlDocument = shared.parse().unwrap();
        let _: club_kdl::kdl::KdlDocument = file.parse().unwrap();
        println!(
            "RUNTIME_SIZE,{case},{},{},{},{}",
            normal_text.len(),
            shared.len(),
            file.len(),
            table.to_kdl().len()
        );
        artifact(&format!("runtime-{case}-shared.kdl"), &shared);
        artifact(&format!("runtime-{case}-file.kdl"), &file);
        artifact("runtime-table.kdl", &table.to_kdl());

        let mut group = c.benchmark_group(format!("name_shortcuts_runtime/{case}"));
        group.throughput(Throughput::Elements(1));
        group.bench_function("serialize/normal", |b| {
            b.iter(|| black_box(club_kdl::to_string(black_box(&value)).unwrap()))
        });
        group.bench_function("serialize/shared", |b| {
            b.iter(|| black_box(table.encode(black_box(&value)).unwrap()))
        });
        group.bench_function("serialize/file", |b| {
            b.iter(|| black_box(compact::to_string(black_box(&value)).unwrap()))
        });
        group.bench_function("deserialize/normal", |b| {
            b.iter(|| {
                black_box(club_kdl::from_str::<normal::Message>(black_box(&normal_text)).unwrap())
            })
        });
        group.bench_function("deserialize/shared", |b| {
            b.iter(|| black_box(table.decode::<normal::Message>(black_box(&shared)).unwrap()))
        });
        group.bench_function("deserialize/file", |b| {
            b.iter(|| black_box(compact::from_str::<normal::Message>(black_box(&file)).unwrap()))
        });
        group.bench_function("build_table", |b| {
            b.iter(|| black_box(NameTable::from_value(black_box(&value)).unwrap()))
        });
        group.finish();
    }
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .sample_size(60)
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(2));
    targets = bench_shortcuts, bench_runtime
}
criterion_main!(benches);
