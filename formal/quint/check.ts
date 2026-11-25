// mem_1ChPFWRE796pYMFmCiUuku / mem_1ChPHcUK47Kmzw3sraumN9
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
const logs = mkdtempSync(join(tmpdir(), "club-kdl-quint-"));
type Model = {
  file: string;
  scenarios: number;
  invariants: readonly string[];
  failures: Record<string, readonly string[]>;
};
// false の設計だけ対応する性質が破れる。集約 invariant もその場合は ng。
const models: Model[] = [
  {
    file: "derive.qnt", scenarios: 10,
    invariants: [
      "writerMatchesBuilder", "propLastWinsAndOrthogonal", "argRoundTrip",
      "enumNewtypeRoundTrip", "stringTokenRoundTrip", "allInvariants",
    ],
    failures: {
      naive: ["argRoundTrip", "enumNewtypeRoundTrip", "stringTokenRoundTrip", "allInvariants"],
      omitNoneArg: ["argRoundTrip", "allInvariants"],
      namedNewtype: ["enumNewtypeRoundTrip", "allInvariants"],
      looseIdent: ["stringTokenRoundTrip", "allInvariants"],
      misorderedArgIndex: ["argRoundTrip", "allInvariants"],
      clubKdlCurrent: [],
    },
  },
  {
    file: "features.qnt", scenarios: 16,
    invariants: [
      "editPreservesUntouchedAndSlashdash", "compactBijectiveAndSafe", "composeDataAndCycleSafe",
      "composeTransformOrderCorrect", "allFeatureInvariants",
    ],
    failures: {
      naiveFeatures: ["editPreservesUntouchedAndSlashdash", "compactBijectiveAndSafe",
        "composeDataAndCycleSafe", "composeTransformOrderCorrect", "allFeatureInvariants"],
      staleEditSpans: ["editPreservesUntouchedAndSlashdash", "allFeatureInvariants"],
      shadowedCompact: ["compactBijectiveAndSafe", "allFeatureInvariants"],
      implicitComposeRef: ["composeDataAndCycleSafe", "allFeatureInvariants"],
      renameBeforeFilter: ["composeTransformOrderCorrect", "allFeatureInvariants"],
      clubKdlFeaturesCurrent: [],
    },
  },
];

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
let moduleCount = 0;
let invariantCount = 0;
let scenarioCount = 0;
for (const { file, scenarios, invariants, failures } of models) {
  const model = fileURLToPath(new URL(`./${file}`, import.meta.url));
  console.log(`\n${file}`);
  for (const main of Object.keys(failures)) {
    moduleCount++;
    const r = quint(`${main}-tests`, ["test", model, `--main=${main}`, `--seed=${seed}`]);
    const count = r.output.match(/\b([1-9][0-9]*) passing\b/);
    const ok = r.status === 0 && !r.error && !r.signal && Number(count?.[1]) === scenarios
      && !/\b(?:failed|failing)\b|Error \[QNT/.test(r.output);
    scenarioCount += Number(count?.[1] ?? 0);
    console.log(`${ok ? "OK" : "NG"} ${main} scenarios: ${count?.[1] ?? "0"}/${scenarios} passing`);
    if (!ok) { failed++; console.error(r.output); }
  }
  console.log("\nmodule                 invariant                          expected actual");
  for (const [main, broken] of Object.entries(failures)) {
    for (const invariant of invariants) {
      invariantCount++;
      const expected = broken.includes(invariant) ? "ng" : "ok";
      const r = quint(`${main}-${invariant}`, [
        "run", model, `--main=${main}`, `--invariant=${invariant}`,
        `--max-samples=${samples}`, `--max-steps=${steps}`, `--seed=${seed}`,
      ]);
      // parse/type/runtime error を反例として数えない。終了コードも確認する。
      const clean = !r.error && !r.signal && !/Error \[QNT/.test(r.output);
      const success = r.output.includes("No violation found");
      const violation = r.output.includes("Invariant violated");
      const actual = clean && r.status === 0 && success && !violation ? "ok"
        : clean && r.status === 1 && violation && !success ? "ng" : "error";
      const ok = actual === expected;
      console.log(`${ok ? "OK" : "NG"} ${main.padEnd(22)} ${invariant.padEnd(34)} ${expected.padEnd(8)} ${actual}`);
      if (!ok) { failed++; console.error(r.output); }
    }
  }
}
console.log(`\n${failed === 0 ? "PASS" : `FAIL (${failed})`}: ${models.length} models / ${moduleCount} modules / ${scenarioCount} scenarios / ${invariantCount} invariant checks. Logs: ${logs}`);
process.exitCode = failed === 0 ? 0 : 1;
