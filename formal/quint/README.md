# derive と本体の設計規則を Quint で確かめる

`mem_1ChPFWRE796pYMFmCiUuku`

`derive.qnt` は、club-kdl のフィールドマッピング、2 つのシリアライズ経路、文字列の型保持を有限のモデルで検査する。設計規則を外すと反例が出ることも検査し、現在の規則が必要な理由を残す。

## 実行

Bun が入った環境で、リポジトリのルートから実行する。

```sh
bun formal/quint/check.ts
```

ランナーは `bunx @informalsystems/quint@0.33.1` を呼ぶ。初回はネットワークからの取得が必要になる。npm 依存・package.json・Cargo の変更は不要。モデルのパスはスクリプト自身から解決するため、別の作業ディレクトリから絶対パスで実行してもよい。

```sh
SAMPLES=5000 STEPS=30 SEED=42 bun formal/quint/check.ts
```

既定は `SAMPLES=2000`、`STEPS=20`、`SEED=20261125`。各モードの `quint test` に続き、6 モード × 6 不変条件の `quint run` を実行する。正常な成功出力と exit 0、または明示的な `Invariant violated` と exit 1 だけを採用する。構文・型・実行エラーやタイムアウトを期待した反例として扱わない。期待と全件一致すれば `PASS` / exit 0、それ以外は非ゼロで終了する。

全コマンド・終了コード・出力（反例の状態列を含む）は、表示される OS の一時ディレクトリ `club-kdl-quint-*` に保存する。ログはリポジトリには生成しない。各コマンドのタイムアウトは 5 分。

個別の反例を見る場合:

```sh
bunx @informalsystems/quint@0.33.1 run formal/quint/derive.qnt \
  --main=omitNoneArg --invariant=argRoundTrip \
  --max-samples=2000 --max-steps=20 --seed=20261125
```

## 比較する 6 モード

| モード | None 引数を null にする | newtype 内側の名前検査を省く | 整合する index | 曖昧な文字列を引用する |
|---|---|---|---|---|
| `naive` | false | false | true | false |
| `omitNoneArg` | false | true | true | true |
| `namedNewtype` | true | false | true | true |
| `looseIdent` | true | true | true | false |
| `misorderedArgIndex` | true | true | false | true |
| `clubKdlCurrent` | true | true | true | true |

`STRICT_ARG_ORDER` は **現行 derive が入力スキーマを検査するフラグではない**。読み書きの index が整合するスキーマを選ぶ、モデル側の前提である。現行コードでも、明示 index と自動採番を非整合に混在させれば `misorderedArgIndex` の反例に相当する。

## 不変条件と期待表

`ok` は指定したサンプル中で反例なし、`ng` は反例が必要という意味。`ng` はランナーの失敗ではない。

| モード | writerMatchesBuilder | propLastWinsAndOrthogonal | argRoundTrip | enumNewtypeRoundTrip | stringTokenRoundTrip | allInvariants |
|---|---|---|---|---|---|---|
| naive | ok | ok | ng | ng | ng | ng |
| omitNoneArg | ok | ok | ng | ok | ok | ng |
| namedNewtype | ok | ok | ok | ng | ok | ng |
| looseIdent | ok | ok | ok | ok | ng | ng |
| misorderedArgIndex | ok | ok | ng | ok | ok | ng |
| clubKdlCurrent | ok | ok | ok | ok | ok | ok |

- `writerMatchesBuilder`: 部品ごとに AST を組んで flatten する経路と、entries / children を別々にたどる直接出力経路のノード構造が一致する。引数順ソートは共通のモデル関数だが、出力の組み立てと flatten は独立に記述する。
- `propLastWinsAndOrthogonal`: 重複プロパティの右端が勝ち、前後にプロパティを挿入しても位置引数列が変わらない。
- `argRoundTrip`: 2 つの Option 引数が往復する。`(None, Some("b"))` を `#null b` と書けば位置が保たれる。None を省略すると後続がずれる。明示 `index=1` の先頭フィールドと自動 `index=0` の次フィールドも、書出し順と読取順が食い違う。
- `enumNewtypeRoundTrip`: unit / named / newtype variant をノード名で選び、newtype の中身を読む。newtype の内側 struct 名は `inner`、外側 variant 名は `wrapped` と異なるため、通常の名前検査では `UnexpectedNode` になる。
- `stringTokenRoundTrip`: 候補文字列を String として書いて再パースしても同じ String に戻る。`-.x` は裸でよいが、`-1`・`.5`・`true` 等は引用が必要。
- `allInvariants`: 以上の論理積。

予約語 `true/false/null/inf/-inf/nan` と `.5/-.5/+.5` は、現行 parser では裸で書くと **構文エラー**になる。`-1/+1` は Integer に変わる。モデルの `Invalid` はパース失敗を表し、KDL の Value の一種ではない。`#true` 等のキーワードトークンと裸の予約語を混同しない。

## 実装との対応

対象はこの変更の起点 `144bee9`。将来の Rust 変更で自動追従する仕組みではないため、対応箇所が変わればモデルも更新する。

| Rust の箇所 | モデル |
|---|---|
| `src/ast.rs`: `Node::args/arg/prop/child` | `argsOf`, `at`, `lookup`, `childArg`（entry 列の順序・重複を保持） |
| `src/ser.rs`: `NodeBuilder::flatten`, Option の値変換 | `serializeTree`, `emitArgs` |
| `derive/src/lib.rs`: `generate_field_serializers` | 明示 index のソート、Option property の省略、child / children / flatten |
| `derive/src/lib.rs`: `generate_field_writers` | `serializeDirect` の entries と children の二巡 |
| `derive/src/lib.rs`: `generate_field_deserializers` | `readArgs`, `required`, `defaulted`, `readStruct` |
| `src/de.rs`: `FromKdlValue`, `KdlDeserialize` | Null・欠落・値変換の抽象、名前付きと unnamed の読取境界 |
| `derive/src/lib.rs`: Data Enum の newtype と `kdl_matches_any_node` | `writeEnum`, `readEnum`, `readChildren` |
| `src/write.rs`: `write_string`, `is_bare_identifier` | `writeString`, `isBare` |
| `src/parse.rs`: `identifier`, `number`, keyword | `parseValueToken` |

10 個のシナリオは基本 struct、flatten の読戻し、重複 property、Option 引数、default / required、明示 index、`arguments` / `unwrap_args`、flatten の引数、Data Enum と全 children の dispatch、引用規則を確かめる。比較用モードでは、欠点が現れることを期待してシナリオを通す。たとえば `namedNewtype` では newtype が `UnexpectedNode` になることを確認する。

## モデルの境界

`init` は単純な正常ケースで開始し、`step` は Option 値、子の有無、flatten の有無、enum variant、文字列、追加プロパティを非決定的に選んで次の入力に移る。永続的な編集履歴をモデル化するものではない。

- ノードは親と葉の 2 階層。flatten のネストは外側・内側の部品を順にたどる有限の列に展開する。無限深さの木や任意のスキーマを証明していない。Quint は再帰型・再帰関数を持たないため、有限構造の fold で表現する（[Quint language basics](https://quint.sh/docs/language-basics)）。
- flatten の往復シナリオは、親と衝突しないプロパティ・子を持つ名前無し struct を対象にする。flatten は親の全引数を再び読むため、一般の任意属性の組合せで往復が保証されるとは主張しない。
- index は自動、0..3。状態探索の主対象は 2 つの Option 引数。全明示 index、必須・default、可変長引数は固定シナリオで追加確認する。`arguments` は既読引数を消費せず全位置引数を集める。
- `Value::String` の有限候補は `CANDIDATES` に列挙する。文字リストによる writer 判定と、候補領域の parser 結果表は独立に記述する。空白・禁止記号・日本語の代表例を含むが、Unicode 全域、escape のバイト列、全数値文法は対象外。
- 直接出力は正規化したノード構造として比較し、KDL のインデント・空白・引用のバイト単位一致を証明していない。シリアライズ経路と文字列トークンの性質は別々に検査する。
- `quint run` は乱択シミュレーション。`No violation found` は全状態網羅や Rust 実行コードの形式証明ではない。乱数 seed は再現用に固定し、必要に応じて別 seed でも検査する。
