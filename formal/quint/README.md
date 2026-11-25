# derive / edit / compact / compose の設計規則を Quint で確かめる

`mem_1ChPFWRE796pYMFmCiUuku` / `mem_1ChPHcUK47Kmzw3sraumN9`

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

既定は `SAMPLES=2000`、`STEPS=20`、`SEED=20261125`。`derive.qnt` の 6 モード × 10 シナリオ・36 不変条件に続いて、`features.qnt` の 6 モード × 16 シナリオ・30 不変条件を実行する（合計 156 シナリオ・66 不変条件チェック）。シナリオの実行数も期待数と一致することを検査する。正常な成功出力と exit 0、または明示的な `Invariant violated` と exit 1 だけを採用する。構文・型・実行エラーやタイムアウトを期待した反例として扱わない。期待と全件一致すれば `PASS` / exit 0、それ以外は非ゼロで終了する。

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

## features.qnt: edit / compact / compose

起点は `8e0915f`。spec [01 名前短縮](../../docs/spec/01-name-shortcuts.ja.md)、[02 短縮参照](../../docs/spec/02-compose-short-references.ja.md)、[03 原文保持編集](../../docs/spec/03-preserve-value-edits.ja.md) を有限の設計モデルに対応させる。Rust の機能実装と既定の出力形式は変更しない。

### 比較する 6 モード

| モード | REPARSE_SPANS_ON_EDIT | NO_ALIAS_SHADOWING | EXPLICIT_SHORT_REF | FILTER_BEFORE_RENAME |
|---|---|---|---|---|
| `naiveFeatures` | false | false | false | false |
| `staleEditSpans` | false | true | true | true |
| `shadowedCompact` | true | false | true | true |
| `implicitComposeRef` | true | true | false | true |
| `renameBeforeFilter` | true | true | true | false |
| `clubKdlFeaturesCurrent` | true | true | true | true |

- `REPARSE_SPANS_ON_EDIT`: 現在の文書に対応する span を使う。false は初回の span を使い続けるため、値を長くした後の編集が別の位置を置換する。
- `NO_ALIAS_SHADOWING`: 名前集合全体を予約してから短い alias を生成し、読込時にも他の元名との衝突を拒否する。false は元名 `a` と `long_name` の alias `a` が重なり、往復で元名が失われる。読込検証では alias が一意でも元名を相互に隠す辞書を比較する。
- `EXPLICIT_SHORT_REF`: 型注釈のない `>` のみを短縮参照とする。false は空データノードも参照扱いする不採用案で、`enabled` や名前 `a` のノードが失われる。`(>)file` と互換エイリアス `(<)file` は両モードともリテラルパスの取込。`pathsAndLiteralTest` は両表記を同じ拡張子なしファイルへ解決・展開し、短縮参照だけに `.kdl` を補う。
- `FILTER_BEFORE_RENAME`: 元の最初の文字列引数に `only` / `except` を適用し、その後 `rename` → `as=`。false は変更後の名前でフィルタを判定する。

### 不変条件期待表

| モード | editPreservesUntouchedAndSlashdash | compactBijectiveAndSafe | composeDataAndCycleSafe | composeTransformOrderCorrect | allFeatureInvariants |
|---|---|---|---|---|---|
| naiveFeatures | ng | ng | ng | ng | ng |
| staleEditSpans | ng | ok | ok | ok | ng |
| shadowedCompact | ok | ng | ok | ok | ng |
| implicitComposeRef | ok | ok | ng | ok | ng |
| renameBeforeFilter | ok | ok | ok | ng | ng |
| clubKdlFeaturesCurrent | ok | ok | ok | ok | ok |

- `editPreservesUntouchedAndSlashdash`: byte 列への span 置換結果と、対象 fragment だけを更新した期待文書全体を比較する。無効化ノード・entry、コメント、他の値を含む全文一致、および失敗直前の原文との一致を検査する。`step` 間で編集履歴を保持する。
- `compactBijectiveAndSafe`: ノード名・プロパティ名の encode → decode、値の不変性、異なる table ID の `TableMismatch`、未知 alias の `UnknownAlias`、辞書読込の衝突拒否を確認する。
- `composeDataAndCycleSafe`: 有限ファイル集合を work list と探索中スタックで辿った結果を、独立に列挙した期待ノード列・エラーと比較する。相対参照は参照元ディレクトリに結合する。ダイヤモンドの D は離脱後に再訪できて 2 回展開され、A → loop → A は `Cycle` になる。探索終了前の燃料切れは成功と扱わない。
- `composeTransformOrderCorrect`: モデル実装を、選択・改名・プレフィックス付加の独立した 3 パスの期待処理と比較する。`only` の無指定と空集合を区別し、`except` と併用する。文字列の第 1 引数を持たない `kdl-version 2` は変更せず残す。
- `allFeatureInvariants`: 上記 4 つの論理積。定数の値自体を不変条件の答えとして使わず、処理結果を比較する。

### シナリオと Rust の対応

| Rust の箇所 | モデル / シナリオ |
|---|---|
| `src/edit.rs::replace_value` と `src/parse.rs::parse_document_with_spans` | `edit`, `offset`: `consecutiveEditTest`（2 回の長さ増加と続く短縮） |
| `src/edit.rs::locate_node`, `entry_count`, 位置引数の `filter/nth`, property の `rposition` | `locate`: `rightmostPropertyTest`, `slashdashAndNestedTest`（右端 property、無効化要素を除いた path / index、子と後続ノード） |
| `src/edit.rs::replace_value` の MissingNode / MissingArgument / MissingProperty | `failedEditsAtomicTest`（長さ変更後の失敗、空 path を含む） |
| `src/compact.rs::NameTable::from_names`, `from_mapping`, `remap` | `table`, `lookup`, `remap`: `shadowingAndRoundTripTest` |
| `src/compact.rs::NameTable::from_node` | `validTable`: `tableValidationTest`（重複元名・重複 alias・元名 shadowing） |
| `src/compact.rs::NameTable::decode_document`, `remap` | `decode`: `tableMismatchTest`, `unknownAliasTest`（node / property の双方） |
| `src/compose/mod.rs::parse_directive`, `short_reference_path` | `directive`, `resolvePath`: `emptyDataNodesTest`, `pathsAndLiteralTest`, `typedAndInvalidReferenceTest` |
| `src/compose/mod.rs::apply_directive`, `resolve`, `process_nodes` | `compose`, `walkStep`: `pathsAndLiteralTest`（サブディレクトリ + cwd の囮ファイル）, `diamondTest`, `cycleTest` |
| `src/compose/mod.rs::transform_node`, `first_string_arg`, `set_first_string_arg` | `transform`, `transformExpected`: `onlyRenamePrefixTest`, `exceptRenamePrefixTest`, `metaPassThroughTest` |

16 シナリオは比較モードの欠点も期待して成功する。たとえば `staleEditSpans` の連続編集テストは、2 回目の編集で全文一致が破れることを確認する。不変条件を直接指定すればその反例の状態列を確認できる。

```sh
bunx @informalsystems/quint@0.33.1 run formal/quint/features.qnt \
  --main=staleEditSpans --invariant=editPreservesUntouchedAndSlashdash \
  --max-samples=2000 --max-steps=20 --seed=20261125
```

### features モデルの境界

- `step` は編集要求・置換値、compact の正常/エラーケース、compose のファイル構成、変換オプションを非決定的に選ぶ。全比較モードが同じ状態遷移・シナリオを使う。
- edit の構文解析は前提。固定文書を ASCII の byte 要素列と fragment に分解し、有効 path を割り当てる。現在の正しい文書の span は fragment 長から再構成するため、Rust parser の再解析・slashdash 認識そのものを証明していない。値は 1〜3 桁の整数表記のみ。UTF-8、escape、raw string、BOM、任意深さは対象外。初回 span が範囲外になる誤実装は原文保持として表し、更新後の期待文書との差を検出する（Rust の panic は再現しない）。
- compact の名前は有限のソート済み集合、alias 候補は `a`〜`e`。短い元名 `a` と長い名前の代表例で、実装の全 Unicode 名・引用時サイズ・alias 繰上がりは扱わない。node / property 名を平坦な 2 レコードに射影し、再帰的な木の走査は捨象する。table ID は辞書そのものの等値で表し、SHA-256 の計算・衝突耐性や KDL envelope parser は検証しない。
- compose のパスは正規化済みディレクトリの要素列。拡張子補完とプレフィックスの文字列操作は有限候補の表で表す。`..`、絶対パス、symlink、OS ごとの path 解釈、glob、実ファイル I/O、任意の参照グラフは対象外。列挙した 8 ケースを最大 40 回の work-list 遷移で処理し、未完了は `FuelExhausted` とする。型付き `>` はデータ、型付き path 引数・複数 path 引数はエラーの代表例として検査するが、全不正構文は網羅しない。
- 変換は `User` / `Order`、改名 `User` → `Acct`、プレフィックス `shared` の有限集合。メタノードは「第 1 引数が文字列ではない」1 ケースに射影する。entry 型注釈の保存・削除や rename 辞書の構築順は対象外。
- [Quint の有限モデル](https://quint.sh/docs/language-basics)に対する乱択シミュレーションであり、Rust 実行コードと自動的に結び付いた証明や全状態網羅ではない。実コードの変更時にはこの対応表・シナリオも見直す。
