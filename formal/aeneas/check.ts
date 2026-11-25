#!/usr/bin/env bun
/** 実 crate の抽出・証明。mutation は隔離コピーにだけ適用する。 */
import { copyFileSync, cpSync, mkdirSync, mkdtempSync, readFileSync, rmSync,
  statSync, symlinkSync, writeFileSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { homedir, tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";

const PROOF = dirname(fileURLToPath(import.meta.url));
const REPO = resolve(PROOF, "../..");
const AENEAS_REV = "2e3d4488280bb8a95411db81019a7854a784861f";
const DATA = process.env.XDG_DATA_HOME ?? join(homedir(), ".local/share");
const TOOLS = resolve(process.env.AENEAS_HOME ?? join(DATA, "nostos-formal/aeneas-2e3d448"));
const METHODS = {
  as_integer: "Value::as_integer", as_float: "Value::as_float",
  as_bool: "Value::as_bool", is_null: "Value::is_null",
  entry_arg: "Entry::arg", entry_value: "Entry::value",
  document_nodes: "Document::nodes", document_as_node: "Document::as_node",
} as const;
type Method = keyof typeof METHODS;
const methods = Object.keys(METHODS) as Method[];

type RunOptions = { cwd: string; capture?: boolean; check?: boolean;
  env?: NodeJS.ProcessEnv };
function run(args: string[], { cwd, capture = false, check = true, env = process.env }: RunOptions) {
  const result = spawnSync(args[0], args.slice(1), { cwd, env,
    encoding: "utf8", stdio: capture ? "pipe" : "inherit", maxBuffer: 32 * 1024 * 1024 });
  const output = (result.stdout ?? "") + (result.stderr ?? "");
  if (result.error || result.signal || (check && result.status !== 0)) {
    throw new Error(`Command failed (${result.error ?? result.signal ?? result.status}): ${args.join(" ")}\n${output}`);
  }
  return { exitCode: result.status, output };
}
function withTempDir(prefix: string, action: (directory: string) => void) {
  const directory = mkdtempSync(prefix);
  try { action(directory); }
  finally { rmSync(directory, { recursive: true, force: true }); }
}
function checkTools() {
  for (const [command, expected] of [
    [[join(TOOLS, "aeneas"), "-version"], "aeneas nightly-2026.11.21-2e3d448"],
    [[join(TOOLS, "charon"), "version"], "0.1.322 (9e193f0317dacc822cc864c1798b117c70dcf7df)"],
    [[join(TOOLS, "charon"), "toolchain-version"], "nightly-2026-09-17"],
  ] as [string[], string][]) {
    const actual = run(command, { cwd: PROOF, capture: true }).output.trim();
    if (actual !== expected) throw new Error(`Tool version mismatch: ${actual}; expected ${expected}`);
  }
  const lean = run(["lake", "env", "lean", "--version"], { cwd: PROOF, capture: true }).output;
  if (!/^Lean \(version 4\.36\.0(?:,|\s)/.test(lean)) throw new Error(`Unexpected Lean version: ${lean}`);
  const manifest: { packages: { name: string; rev: string }[] } =
    JSON.parse(readFileSync(join(PROOF, "lake-manifest.json"), "utf8"));
  if (manifest.packages.find((p) => p.name === "aeneas")?.rev !== AENEAS_REV) {
    throw new Error("Aeneas executable and Lean dependency revisions must match");
  }
  const checkout = run(["git", "rev-parse", "HEAD"], {
    cwd: join(PROOF, ".lake/packages/aeneas"), capture: true,
  }).output.trim();
  if (checkout !== AENEAS_REV) throw new Error(`Aeneas Lean checkout mismatch: ${checkout}`);
}
function extract(sourceRoot: string, proofRoot: string) {
  const cache = join(proofRoot, ".lake");
  mkdirSync(cache, { recursive: true });
  const llbc = join(cache, "club_kdl.llbc");
  rmSync(llbc, { force: true });
  run([join(TOOLS, "charon"), "cargo", "--preset=aeneas", "--sysroot", "default",
    ...methods.flatMap((method) => ["--start-from", `crate::ast::${METHODS[method]}`]),
    "--dest-file", llbc, "--", "--manifest-path", "Cargo.toml", "--lib",
    ...(statSync(join(sourceRoot, "Cargo.lock"), { throwIfNoEntry: false }) ? ["--locked"] : [])],
    { cwd: sourceRoot, env: { ...process.env, CARGO_TARGET_DIR: join(cache, "rust-target") } });
  if (!statSync(llbc, { throwIfNoEntry: false })?.isFile()) throw new Error("No fresh Charon LLBC");
  withTempDir(join(cache, "extract-"), (generated) => {
    run([join(TOOLS, "aeneas"), "-backend", "lean", "-namespace", "ClubKdlGenerated",
      "-dest", generated, "-impl-namespace", "-use-lean-modules", "false", "-abort-on-error",
      "-warnings-as-errors", "-no-progress-bar", llbc], { cwd: sourceRoot });
    const raw = readFileSync(join(generated, "ClubKdl.lean"), "utf8");
    // Fixed Aeneas emits F64 without a Lean carrier. Only add the import;
    // every extracted declaration stays byte-for-byte intact.
    const anchor = "import Aeneas\n";
    if (raw.split(anchor).length !== 2) throw new Error("Aeneas import layout changed");
    writeFileSync(join(cache, "ClubKdl.raw.lean"), raw);
    writeFileSync(join(proofRoot, "ClubKdl.lean"), raw.replace(anchor, `${anchor}import ExtractionCompat\n`));
  });
}
function prove(proofRoot: string, display = true) {
  run(["lake", "build", "ClubKdl"], { cwd: proofRoot });
  const result = run(["lake", "env", "lean", "-DwarningAsError=true", "ClubKdlProof.lean"],
    { cwd: proofRoot, capture: true, check: false });
  if (display) process.stdout.write(result.output);
  return result;
}
function proofAxioms(output: string, method: Method): string[] {
  const theorem = `ClubKdlProof.${method}_matches_spec`;
  const reports = [...output.matchAll(new RegExp(
    `'${theorem.replaceAll(".", "\\.")}' (?:depends on axioms: \\[([^\\]]*)\\]|does not depend on any axioms)`, "g"))];
  if (reports.length !== 1) throw new Error(`Expected one axiom report for ${theorem}`);
  return (reports[0][1] ?? "").split(",").map((a) => a.trim()).filter(Boolean);
}
function checkAxioms(output: string, selected: Method[] = methods) {
  const allowed = new Set(["propext", "Quot.sound"]);
  for (const method of selected) {
    const axioms = proofAxioms(output, method);
    if (axioms.some((a) => !allowed.has(a))) throw new Error(`Unexpected axioms for ${method}: ${axioms}`);
  }
}
function mutationTest() {
  const mutations: Record<Method, [string, string]> = {
    as_integer: ["Value::Integer(i) => Some(*i),", "Value::Integer(_i) => None,"],
    as_float: ["Value::Float(f) => Some(*f),", "Value::Float(_f) => None,"],
    as_bool: ["Value::Bool(b) => Some(*b),", "Value::Bool(_b) => None,"],
    is_null: ["matches!(self, Value::Null)", "matches!(self, Value::Bool(_))"],
    entry_arg: ["pub fn arg(value: Value<'a>) -> Self {\n        Self {\n            name: None,\n            ty: None,\n            value,",
      "pub fn arg(_value: Value<'a>) -> Self {\n        Self {\n            name: None,\n            ty: None,\n            value: Value::Null,"],
    entry_value: ["pub fn value(&self) -> &Value<'a> {\n        &self.value",
      "pub fn value(&self) -> &Value<'a> {\n        &Value::Null"],
    document_nodes: ["pub fn nodes(&self) -> &[Node<'a>] {\n        &self.root.children",
      "pub fn nodes(&self) -> &[Node<'a>] {\n        &self.root.children[0..0]"],
    document_as_node: ["pub fn as_node(&self) -> &Node<'a> {\n        &self.root",
      "pub fn as_node(&self) -> &Node<'a> {\n        &self.root.children[0]"],
  };
  const sourceFile = join(REPO, "src/ast.rs");
  const originalSource = readFileSync(sourceFile);
  const originalGenerated = readFileSync(join(PROOF, "ClubKdl.lean"));
  try {
    withTempDir(join(tmpdir(), "club-kdl-rust-mutation-"), (root) => {
      // Cargo validates workspace members and explicit bench paths even for --lib.
      for (const name of ["Cargo.toml", "Cargo.lock"]) copyFileSync(join(REPO, name), join(root, name));
      for (const name of ["src", "derive", "codegen", "benches"]) {
        cpSync(join(REPO, name), join(root, name), { recursive: true, dereference: true,
          filter: (path) => !["target", ".git"].includes(path.split(/[\\/]/).at(-1)!) });
      }
      const source = join(root, "src/ast.rs");
      const trial = join(root, "proof");
      mkdirSync(trial);
      for (const name of ["lakefile.toml", "lake-manifest.json", "lean-toolchain", "ClubKdlProof.lean", "ExtractionCompat.lean"]) {
        copyFileSync(join(PROOF, name), join(trial, name));
      }
      mkdirSync(join(trial, ".lake"));
      // 依存のビルドキャッシュだけ共有。対象の Lean/Rust 成果物は隔離する。
      symlinkSync(join(PROOF, ".lake/packages"), join(trial, ".lake/packages"), "dir");
      for (const method of methods) {
        const [from, to] = mutations[method];
        const text = originalSource.toString();
        if (text.split(from).length !== 2) throw new Error(`Mutation target changed: ${method}`);
        writeFileSync(source, text.replace(from, to));
        console.log(`MUTATION: ${method} (temporary Rust copy)`);
        extract(root, trial);
        if (readFileSync(join(trial, "ClubKdl.lean")).equals(originalGenerated)) {
          throw new Error(`Mutation did not reach Lean: ${method}`);
        }
        const result = prove(trial, false);
        const log = join(PROOF, `.lake/mutation-${method}.log`);
        writeFileSync(log, result.output);
        if (result.exitCode !== 1 || !/ClubKdlProof\.lean:\d+:\d+: error:/.test(result.output)
          || !proofAxioms(result.output, method).includes("sorryAx")) {
          throw new Error(`Expected a proof failure, not a tool failure: ${method}; see ${log}`);
        }
        checkAxioms(result.output, methods.filter((other) => other !== method));
        console.log(`PASS: ${method} rejected; other 7 specifications still proved (log: ${log})`);
      }
    });
  } finally {
    if (!readFileSync(sourceFile).equals(originalSource)
      || !readFileSync(join(PROOF, "ClubKdl.lean")).equals(originalGenerated)) {
      throw new Error("Mutation test changed the original Rust or generated Lean");
    }
  }
  console.log("PASS: all 8 mutations rejected; original src/ast.rs and ClubKdl.lean byte-identical");
}
function main() {
  const { values } = parseArgs({ args: process.argv.slice(2), options: {
    "mutation-test": { type: "boolean" }, help: { type: "boolean", short: "h" },
  }, strict: true, allowPositionals: false });
  if (values.help) {
    console.log("Usage: bun formal/aeneas/check.ts [--mutation-test]\n実 crate を再抽出・証明。mutation は一時コピーだけを変更。");
    return;
  }
  checkTools();
  extract(REPO, PROOF);
  const result = prove(PROOF);
  writeFileSync(join(PROOF, ".lake/proof.log"), result.output);
  if (result.exitCode !== 0) throw new Error(`Lean proof failed (exit ${result.exitCode})`);
  checkAxioms(result.output);
  console.log("PASS: current Rust matches all 8 specifications; axiom audit passed");
  if (values["mutation-test"]) mutationTest();
}
try { main(); }
catch (error) {
  console.error(error instanceof Error ? error.message : String(error));
  process.exitCode = 1;
}
