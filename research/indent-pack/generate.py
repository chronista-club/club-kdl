"""Generate research writers from the production writer; no production edits.

Only indentation and structural delimiters change. Quoting, value formatting,
and identifier rules are copied verbatim. Run before cargo fmt.
"""
from pathlib import Path
import re

here = Path(__file__).resolve().parent
root = here.parent.parent
writer = (root / "src/write.rs").read_text().split("#[cfg(test)]")[0]
parse = (root / "src/parse.rs").read_text()
predicates = parse[parse.index("/// ASCII bytes"):parse.index("impl<'a> Parser<'a>")]
no_indent, removed = re.subn(
    r"for _ in 0\.\.(?:self\.)?depth \{\s*(?:self\.)?out\.push_str\(INDENT\);\s*\}",
    "", writer,
)
assert removed == 4, removed
no_indent = no_indent.replace('const INDENT: &str = "    ";', "")
no_indent = no_indent.replace("node: &Node<'_>, depth: usize)", "node: &Node<'_>, _depth: usize)")
no_indent = no_indent.replace("write_node(out, child, depth + 1)", "write_node(out, child, _depth + 1)")
single_line = no_indent.replace('out.push_str(" {\\n");', "out.push('{');").replace("out.push('\\n');", "out.push(';');")
for name, source in [("baseline", writer), ("no_indent", no_indent), ("single_line", single_line), ("parse", predicates)]:
    (here / "src" / f"{name}.rs").write_text("// Generated research copy; see generate.py.\n" + source)
