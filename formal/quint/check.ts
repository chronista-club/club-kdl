// mem_1ChPFWRE796pYMFmCiUuku
// Bun 実行。依存追加なし。全出力は一時ディレクトリに残す。
import { spawnSync } from "node:child_process";
import { mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

function positiveInteger(name: string, fallback: number): number {
  const value = process.env[name];
  if (value === undefined) return fallback;
  if (!/^[1-9][0-9]*$/.test(value) || !Number.isSafeInteger(Number(value))) {
    throw new Error(`${name} は正の安全な整数で指定してください: ${value}`);
  }
  return Number(value);
}

const samples = positiveInteger("SAMPLES", 2000);
const steps = positiveInteger("STEPS", 20);
const seed = process.env.SEED ?? "20261125";
if (!/^(0x[0-9a-f]+|[0-9]+)$/i.test(seed)) throw new Error("SEED は整数で指定してください");
const model = fileURLToPath(new URL("./derive.qnt", import.meta.url));
const logs = mkdtempSync(join(tmpdir(), "club-kdl-quint-"));
const invariants = [
  "writerMatchesBuilder", "propLastWinsAndOrthogonal", "argRoundTrip",
  "enumNewtypeRoundTrip", "stringTokenRoundTrip", "allInvariants",
] as const;
type Invariant = typeof invariants[number];
// false の設計だけ個別の性質が破れる。allInvariants もその場合は ng。
const failures: Record<string, readonly Invariant[]> = {
  naive: ["argRoundTrip", "enumNewtypeRoundTrip", "stringTokenRoundTrip", "allInvariants"],
  omitNoneArg: ["argRoundTrip", "allInvariants"],
  namedNewtype: ["enumNewtypeRoundTrip", "allInvariants"],
  looseIdent: ["stringTokenRoundTrip", "allInvariants"],
  misorderedArgIndex: ["argRoundTrip", "allInvariants"],
  clubKdlCurrent: [],
};

function quint(label: string, args: string[]) {
  const result = spawnSync("bunx", ["@informalsystems/quint@0.33.1", ...args], {
    encoding: "utf8", timeout: 300_000, maxBuffer: 16 * 1024 * 1024,
  });
  const output = `${result.stdout ?? ""}\n${result.stderr ?? ""}`;
  writeFileSync(join(logs, `${label}.log`), [
    `bunx @informalsystems/quint@0.33.1 ${args.join(" ")}`,
    `exit=${result.status} signal=${result.signal} error=${result.error?.message ?? "none"}`,
    output,
  ].join("\n"));
  return { output, status: result.status, error: result.error, signal: result.signal };
}

console.log(`Quint 0.33.1 / samples=${samples} steps=${steps} seed=${seed}\nLogs: ${logs}`);
const version = quint("version", ["--version"]);
if (version.status !== 0 || version.error || !/^0\.33\.1$/m.test(version.output)) {
  console.error(version.output);
  throw new Error("Quint 0.33.1 を起動できません");
}
let failed = 0;
for (const main of Object.keys(failures)) {
  const r = quint(`${main}-tests`, ["test", model, `--main=${main}`, `--seed=${seed}`]);
  const count = r.output.match(/\b([1-9][0-9]*) passing\b/);
  const ok = r.status === 0 && !r.error && !r.signal && count !== null
    && !/\b(?:failed|failing)\b|Error \[QNT/.test(r.output);
  console.log(`${ok ? "OK" : "NG"} ${main} scenarios: ${count?.[1] ?? "0"} passing`);
  if (!ok) { failed++; console.error(r.output); }
}
console.log("\nmodule                 invariant                      expected actual");
for (const [main, broken] of Object.entries(failures)) {
  for (const invariant of invariants) {
    const expected = broken.includes(invariant) ? "ng" : "ok";
    const r = quint(`${main}-${invariant}`, [
      "run", model, `--main=${main}`, `--invariant=${invariant}`,
      `--max-samples=${samples}`, `--max-steps=${steps}`, `--seed=${seed}`,
    ]);
    // 終了コードも確認する。parse/type/runtime error を反例として数えない。
    const clean = !r.error && !r.signal && !/Error \[QNT/.test(r.output);
    const success = r.output.includes("No violation found");
    const violation = r.output.includes("Invariant violated");
    const actual = clean && r.status === 0 && success && !violation ? "ok"
      : clean && r.status === 1 && violation && !success ? "ng" : "error";
    const ok = actual === expected;
    console.log(`${ok ? "OK" : "NG"} ${main.padEnd(22)} ${invariant.padEnd(30)} ${expected.padEnd(8)} ${actual}`);
    if (!ok) { failed++; console.error(r.output); }
  }
}
console.log(`\n${failed === 0 ? "PASS" : `FAIL (${failed})`}: 6 modules / 36 invariant checks. Logs: ${logs}`);
process.exitCode = failed === 0 ? 0 : 1;
