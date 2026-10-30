"""Export Criterion estimates with their 95% confidence intervals."""
import csv
import json
from pathlib import Path

here = Path(__file__).resolve().parent
rows = []
for path in sorted((here / "target/criterion").glob("**/new/estimates.json")):
    metadata = json.loads((path.parent / "benchmark.json").read_text())
    estimates = json.loads(path.read_text())
    estimator = "slope" if estimates.get("slope") else "mean"
    result = estimates[estimator]
    ci = result["confidence_interval"]
    rows.append([metadata["full_id"], estimator, result["point_estimate"], ci["lower_bound"], ci["upper_bound"], ci["confidence_level"]])
assert len(rows) == 90, f"Expected 90 results, got {len(rows)}"
with (here / "artifacts/results.csv").open("w") as output:
    writer = csv.writer(output, lineterminator="\n")
    writer.writerow(["benchmark", "estimator", "ns_per_iteration", "ci_low_ns", "ci_high_ns", "confidence"])
    writer.writerows(rows)
lookup = {row[0]: row[2] for row in rows}
print("case,operation,pretty_us,no_indent_us,single_line_us,no_indent_speedup")
for case in ["depth0_width100", "depth1_width100", "depth8_width100", "depth32_width100", "depth128_width1"]:
    for op in ["write_fresh", "write_reuse", "parse"]:
        values = [lookup[f"{case}/aliases/{op}/{mode}"] / 1000 for mode in ["pretty", "no_indent", "single_line"]]
        print(f"{case},{op},{values[0]:.3f},{values[1]:.3f},{values[2]:.3f},{values[0]/values[1]:.3f}")
