"""Compare the historical and current codecs inside a temporary crate copy.

Neither historical private-field access nor benchmark-only modules enter the
production library. Invoke from any directory; results remain under target/.
"""
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

here = Path(__file__).resolve().parent
root = here.parent.parent
with tempfile.TemporaryDirectory(prefix="club-kdl-codec-") as directory:
    scratch = Path(directory)
    for name in ["src", "derive", "codegen", "benches"]:
        shutil.copytree(root / name, scratch / name, ignore=shutil.ignore_patterns("target", ".git", "node_modules"))
    for name in ["Cargo.toml", "Cargo.lock", "README.md"]:
        shutil.copyfile(root / name, scratch / name)
    for name in ["baseline_compact", "codec_comparison"]:
        source = here / ("src/baseline_compact.rs" if name == "baseline_compact" else "codec_comparison.rs")
        shutil.copyfile(source, scratch / "src" / f"{name}.rs")
    with (scratch / "src/lib.rs").open("a") as output:
        output.write("\nextern crate self as club_kdl;\n#[cfg(test)]\nmod baseline_compact;\n#[cfg(test)]\nmod codec_comparison;\n")
    env = dict(os.environ)
    env["CARGO_TARGET_DIR"] = str(here / "target/paired-codec-build")
    env["KDL_CODEC_RESULTS"] = str(here / "target/paired-codec-results")
    subprocess.run(["cargo", "test", "-p", "club-kdl", "--lib", "--release", "--locked", "codec_comparison::paired_benchmark", "--", "--ignored", "--nocapture"], cwd=scratch, env=env, check=True)
