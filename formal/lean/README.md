# AST・derive・edit・compact・compose の構造法則（Lean 4）

`KdlCore.lean` は `club-kdl` 本体と `club-kdl-derive` の構造を手書きでモデル化し、任意の入力について法則を証明します。Rust 実装を直接抽出する検証は [../aeneas](../aeneas/README.md) に分けています。

## 実行

Lean は `lean-toolchain` で **4.36.0** に固定しています。elan が利用可能な環境で実行します。外部 Lake パッケージは不要です。

```sh
cd formal/lean
lean -DwarningAsError=true KdlCore.lean
```

終了コード 0 に加えて、27 個の構造定理と 15 個の補助定理、計 42 個すべての `#print axioms` が `does not depend on any axioms` と報告することを確認します。`sorry`、追加の `axiom`、`Classical.choice`、`propext`、`Quot.sound` は証明に使いません。Lean 標準の一部の List 補題は `propext` に依存するため、必要な補題も構成的に証明しています。

## 証明している法則

| 定理 | 内容・Rust との対応 |
| --- | --- |
| `args_insert_prop`, `arg_insert_prop` | `src/ast.rs` の `Node::args` / `arg`。任意の二つのリストの間に property を追加しても全位置引数と任意の添字の値が不変。先頭・末尾も含む |
| `args_append_prop`, `arg_append_prop` | 末尾追加の明示的な系 |
| `prop_insert_arg`, `prop_append_arg` | 任意位置・末尾への位置引数追加はすべての property 検索に影響しない |
| `prop_append_same`, `prop_append_ne` | `Node::prop` と同じ `entries.reverse.find?` による右端優先。同じキーは追加値が勝ち、異なるキーは保存 |
| `option_arg_roundtrip` | `src/ser.rs` の `Option<T>::to_kdl_value` と `derive/src/lib.rs` の引数デコード。`None` は `#null` として一つのスロットを占め、`filter (!is_null)` で戻る |
| `option_args_indices`, `option_args_roundtrip_at`, `option_args_length` | 任意個の Option 引数について、要素数・各添字・往復の一致を保存。後続引数がずれない |
| `omitting_none_shifts_index` | `None` を省略する誤実装では `(none, some v)` の `v` が添字 0 にずれ、添字 1 が欠ける反例（任意の非 null 値） |
| `option_prop_roundtrip` | Option property は `None` なら entry 自体を省略し、`Some v` なら `key=v` を出力して読む |
| `writer_eq_builder` | `src/ser.rs` の `NodeBuilder::flatten` と、derive が `src/write.rs` の `NodeWriter` に送る二段階の走査の `(entries, children)` が一致 |
| `newtype_unnamed_roundtrip`, `newtype_named_fails` | enum newtype が名前を variant 名に上書きしても、外側で分岐し内側で `__from_kdl_node_unnamed` を呼べば往復する。内側でも名前検査をすると、異名の場合は失敗 |

### edit / compact / compose

| 定理 | 内容・Rust との対応 |
| --- | --- |
| `locate_node_matches_dfs_entry` | `src/edit.rs` の `entry_count` / `locate_node`。任意深さ・分岐の木で、パスから得た `offset + idx` が独立に生成した DFS entry 列の目的の値を指す |
| `remap_argument_unchanged` | 辞書が何であっても位置引数の値・型注釈は無変更 |
| `remap_entry_roundtrip`, `remap_entries_roundtrip` | エンコードに成功した property 名・値・型注釈と列の順序がデコードで戻る |
| `remap_node_roundtrip`, `remap_nodes_roundtrip` | ノード名・entries・任意深さの children 全体が戻る |
| `shadowing_breaks_roundtrip` | 異なる二つの元名が同じ alias を使うと、少なくとも片方の往復が壊れる |
| `transform_preserves_nameless` | `arg(0)` が文字列でないノードは、任意の only / except / rename / prefix の下で無変更 |
| `transform_filters_on_original_name` | フィルタは元名だけで判定し、その後 rename → prefix。変更後の名前のフィルタ条件は不要 |
| `rename_filter_noncommute` | rename 後の名前が only / except で拒否される場合、先に rename すると除外されるが、正しい順序なら通過する |

`EditNode` はパーサーが返す active な entries と children を持ちます。`dfsEntries` は各ノードの entries → children → 後続兄弟、`entry_count` は独立に再帰加算、`locate_node` は先行兄弟の subtree と親 entries を加算して降ります。`locate_in_node` はこの最後の降下を構造再帰に分けた補助です。空パス・範囲外のパスは `none` です。

`RemapNode` / `RemapEntry` は名前・型注釈・値・children を区別します。辞書 `encode` / `decode` の前提は `encode s = some a → decode a = some s` だけです。すべての名前が登録済みとは仮定せず、**エンコードに成功した**入力について往復を証明します。辞書にない名前は `none` です。元の Rust 同様、位置引数と型注釈の内容は辞書で変換しません。

compose の文字列は `ComposeName := List Char` で表し、`prefixed` は `prefix ++ ['.'] ++ name` です。Lean 4.36 の最適化された `String.append` / `BEq String` 自体の公理依存を構造証明に持ち込まないため、文字列の構成的な表現を使います。`firstEntryString` は property を飛ばしますが、最初の位置引数が整数等ならそこで `none` になります。後ろの文字列引数を名前と誤認しません。setter は Rust の `Entry::arg` と同様に、置換した文字列引数の型注釈を消します。

`Val` は null・整数・浮動小数値のビット列・bool・文字列を区別します。添字アクセス `nth` は Rust の `Iterator::nth` の構造モデルです。null 除去も標準ライブラリの補題を経由せず同じ分岐を直接記述しています。

Option の往復には `NonNull opt` を前提とします。`Some Val.null` と `None` はともに `#null` になり区別できないため、この前提を外した往復は主張しません。property の往復は、そのフィールドから生成した entry 列に対するものです。既存の同名 property を残したまま `None` を追記しても、それを削除する効果はありません。

`FlattenTree.fields` は各 struct が生成した、既に所定順序に並んだ entries と children を表します。`flatten parent nested` を任意にネストさせることで、複数・多段の flatten を表現します。builder は二つの列を一緒に蓄積し、writer は entries と children を独立に再帰走査します。子ノードは任意の型 `Child` として扱い、列の順序・重複・中身を保持します。

## 保証の範囲と信頼境界

- これは **手書き構造モデルの定理**です。Rust の parser、proc-macro 展開、型変換、エラー伝播の全体を抽出した証明ではありません。対応する Rust が変わった場合はモデルとの対応を再レビューします。
- `writer_eq_builder` が示すのは entries と children の列の一致です。文字列エスケープ、インデント、数値表記、I/O、メモリ使用量、実際の文字列全体の一致はここでは証明していません。
- newtype の payload は変換済みの値として扱います。内側の値変換が失敗するケースや、ユーザー実装の serializer/deserializer の正しさは対象外です。内側に入る前の variant 名の分岐と、内側の名前検査の有無をモデル化しています。
- edit の DFS は active entry の順序・添字の対応です。パーサーの span バイト境界、slashdash の除外処理、Rust `usize` 加算のオーバーフローがないこと自体は抽出証明していません。
- compact の往復は名前変換の構造法則です。ヘッダ構文解析、辞書構築、alias 採番、エラーの種類や途中失敗時の状態は証明の対象外です。
- compose の文字列モデルと Rust / Lean の UTF-8 実装の同値性、ファイル読込・glob・循環参照はここでは証明しません。
- Lean カーネルによる型検査を信頼します。空公理は数学的な仮定を追加していないという意味であり、モデルと Rust の対応そのものが自動証明されたという意味ではありません。
- Rust の公開 API、挙動、Cargo 依存は変更しません。既存の Rust テストと、実抽出した限定範囲の証明を併用します。

## CI

`.github/workflows/ci.yml` の `Lean proofs` は上記コマンドを実行し、`depends on axioms:` / `sorryAx` がないことと、全定理宣言数に対する空公理レポート数の一致を検査します。その後 [Aeneas の 15 メソッドと 15 mutation](../aeneas/README.md) を検証し、再生成物・依存 lock の差分ゼロも確認します。`Quint verify` は別ジョブで `bun formal/quint/check.ts` を実行します。
