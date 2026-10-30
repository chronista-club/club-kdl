//! Same-process comparison of actual old/new typed compact APIs.
use crate::baseline_compact as before;
use crate::{KdlDeserialize, KdlSerialize, compact};
use criterion::Criterion;
use std::{hint::black_box, time::Duration};

#[derive(Debug, PartialEq, KdlSerialize, KdlDeserialize)]
#[kdl(name = "object_message")]
struct Message {
    #[kdl(property)]
    sequence_number: u64,
    #[kdl(children)]
    records: Vec<Record>,
}

#[derive(Debug, PartialEq, KdlSerialize, KdlDeserialize)]
#[kdl(name = "record_update")]
struct Record {
    #[kdl(property)]
    object_identifier: String,
    #[kdl(property)]
    position_horizontal: f64,
    #[kdl(property)]
    visible: bool,
}

fn codec(c: &mut Criterion) {
    let value = Message {
        sequence_number: 42,
        records: (0..100)
            .map(|i| Record {
                object_identifier: format!("object-{i}"),
                position_horizontal: 12.5,
                visible: i % 2 == 0,
            })
            .collect(),
    };
    let old_table = before::NameTable::from_value(&value).unwrap();
    let new_table = compact::NameTable::from_value(&value).unwrap();
    assert_eq!(old_table.id(), new_table.id());
    let old_wire = old_table.encode(&value).unwrap();
    let new_wire = new_table.encode(&value).unwrap();
    let old_file = before::to_string(&value).unwrap();
    let new_file = compact::to_string(&value).unwrap();
    // Both versions can read both layouts, including the complete typed object.
    for wire in [&old_wire, &new_wire] {
        assert_eq!(old_table.decode::<Message>(wire).unwrap(), value);
        assert_eq!(new_table.decode::<Message>(wire).unwrap(), value);
    }
    for file in [&old_file, &new_file] {
        assert_eq!(before::from_str::<Message>(file).unwrap(), value);
        assert_eq!(compact::from_str::<Message>(file).unwrap(), value);
    }
    println!(
        "PAIRED_SIZE,shared,{},{},file,{},{}",
        old_wire.len(),
        new_wire.len(),
        old_file.len(),
        new_file.len()
    );
    let mut group = c.benchmark_group("paired_codec/batch100");
    group.bench_function("serialize_shared/before", |b| {
        b.iter(|| black_box(old_table.encode(black_box(&value)).unwrap()))
    });
    group.bench_function("serialize_shared/after", |b| {
        b.iter(|| black_box(new_table.encode(black_box(&value)).unwrap()))
    });
    group.bench_function("deserialize_shared/before", |b| {
        b.iter(|| black_box(old_table.decode::<Message>(black_box(&old_wire)).unwrap()))
    });
    group.bench_function("deserialize_shared/after", |b| {
        b.iter(|| black_box(new_table.decode::<Message>(black_box(&new_wire)).unwrap()))
    });
    group.bench_function("serialize_file/before", |b| {
        b.iter(|| black_box(before::to_string(black_box(&value)).unwrap()))
    });
    group.bench_function("serialize_file/after", |b| {
        b.iter(|| black_box(compact::to_string(black_box(&value)).unwrap()))
    });
    group.bench_function("deserialize_file/before", |b| {
        b.iter(|| black_box(before::from_str::<Message>(black_box(&old_file)).unwrap()))
    });
    group.bench_function("deserialize_file/after", |b| {
        b.iter(|| black_box(compact::from_str::<Message>(black_box(&new_file)).unwrap()))
    });
    group.finish();
}

#[test]
#[ignore = "manual performance experiment; run research/indent-pack/run-codec.py"]
fn paired_benchmark() {
    let mut c = Criterion::default()
        .sample_size(60)
        .warm_up_time(Duration::from_millis(300))
        .measurement_time(Duration::from_secs(2))
        .nresamples(10_000)
        .output_directory(&std::path::PathBuf::from(
            std::env::var("KDL_CODEC_RESULTS").unwrap(),
        ));
    codec(&mut c);
    c.final_summary();
}
