use club_kdl::{Document, Entry, Node, Value, compact::NameTable};
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use indent_pack_research::{Layout, write, write_into};
use std::{hint::black_box, time::Duration};

fn fixture(depth: usize, width: usize) -> Document<'static> {
    let mut nodes = (0..width)
        .map(|i| {
            let mut n = Node::new("record_update");
            for (key, value) in [
                ("object_identifier", Value::Integer(i as i128)),
                ("position_horizontal", Value::Float(12.5)),
                ("position_vertical", Value::Float(-3.25)),
                ("rotation_degrees", Value::Float(90.0)),
                (
                    "display_name",
                    Value::String(format!("object-{i:03}").into()),
                ),
                ("content_payload", Value::String("  x\n\t{value};  ".into())),
            ] {
                n.entries_mut().push(Entry::prop(key, value));
            }
            n
        })
        .collect::<Vec<_>>();
    for _ in 0..depth {
        let mut parent = Node::new("group_layer");
        parent.children_mut().extend(nodes);
        nodes = vec![parent];
    }
    Document::from_nodes(nodes)
}

fn layout(c: &mut Criterion) {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("artifacts");
    std::fs::create_dir_all(&dir).unwrap();
    let mut sizes = String::from("case,wire,pretty_bytes,no_indent_bytes,single_line_bytes\n");
    for (depth, width) in [(0, 100), (1, 100), (8, 100), (32, 100), (128, 1)] {
        let case = format!("depth{depth}_width{width}");
        let original = fixture(depth, width);
        let table = NameTable::from_document(&original);
        let shared = table.encode_document(original.clone()).unwrap();
        let alias = Document::parse(&shared).unwrap();
        for (wire, doc) in [("original", &original), ("aliases", &alias)] {
            let pretty = doc.to_string();
            let mut lengths = vec![];
            let mut group = c.benchmark_group(format!("{case}/{wire}"));
            for (name, mode) in [
                ("pretty", Layout::Pretty),
                ("no_indent", Layout::NoIndent),
                ("single_line", Layout::SingleLine),
            ] {
                let text = write(doc, mode);
                assert_eq!(Document::parse(&text).unwrap().to_string(), pretty);
                let _: club_kdl::kdl::KdlDocument = text.parse().unwrap();
                if wire == "aliases" {
                    assert_eq!(table.decode_document(&text).unwrap(), original);
                }
                lengths.push(text.len());
                std::fs::write(dir.join(format!("{case}-{wire}-{name}.kdl")), &text).unwrap();
                group.bench_function(BenchmarkId::new("write_fresh", name), |b| {
                    b.iter(|| black_box(write(black_box(doc), mode)))
                });
                let mut buffer = String::with_capacity(pretty.len());
                group.bench_function(BenchmarkId::new("write_reuse", name), |b| {
                    b.iter(|| {
                        buffer.clear();
                        write_into(black_box(doc), mode, &mut buffer);
                        black_box(&buffer);
                    })
                });
                group.bench_function(BenchmarkId::new("parse", name), |b| {
                    b.iter(|| black_box(Document::parse(black_box(&text)).unwrap()))
                });
            }
            group.finish();
            sizes.push_str(&format!(
                "{case},{wire},{},{},{}\n",
                lengths[0], lengths[1], lengths[2]
            ));
        }
    }
    std::fs::write(dir.join("sizes.csv"), sizes).unwrap();
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(40).warm_up_time(Duration::from_millis(200)).measurement_time(Duration::from_secs(1)).nresamples(10_000);
    targets = layout
}
criterion_main!(benches);
