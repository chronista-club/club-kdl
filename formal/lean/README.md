# AST・derive の構造法則（Lean 4）

`KdlCore.lean` は `club-kdl` 本体と `club-kdl-derive` の構造を手書きでモデル化し、任意の入力について法則を証明します。Rust 実装を直接抽出する検証は [../aeneas](../aeneas/README.md) に分けています。

## 実行

Lean は `lean-toolchain` で **4.36.0** に固定しています。elan が利用可能な環境で実行します。外部 Lake パッケージは不要です。

```sh
cd formal/lean
lean -DwarningAsError=true KdlCore.lean
```

終了コード 0 に加えて、17 個の構造定理と 7 個の補助定理、計 24 個すべての `#print axioms` が `does not depend on any axioms` と報告することを確認します。`sorry`、追加の `axiom`、`Classical.choice`、`propext`、`Quot.sound` は証明に使いません。Lean 標準の一部の List 補題は `propext` に依存するため、必要な補題も構成的に証明しています。

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

`Val` は null・整数・浮動小数値のビット列・bool・文字列を区別します。添字アクセス `nth` は Rust の `Iterator::nth` の構造モデルです。null 除去も標準ライブラリの補題を経由せず同じ分岐を直接記述しています。

Option の往復には `NonNull opt` を前提とします。`Some Val.null` と `None` はともに `#null` になり区別できないため、この前提を外した往復は主張しません。property の往復は、そのフィールドから生成した entry 列に対するものです。既存の同名 property を残したまま `None` を追記しても、それを削除する効果はありません。

`FlattenTree.fields` は各 struct が生成した、既に所定順序に並んだ entries と children を表します。`flatten parent nested` を任意にネストさせることで、複数・多段の flatten を表現します。builder は二つの列を一緒に蓄積し、writer は entries と children を独立に再帰走査します。子ノードは任意の型 `Child` として扱い、列の順序・重複・中身を保持します。

## 保証の範囲と信頼境界

- これは **手書き構造モデルの定理**です。Rust の parser、proc-macro 展開、型変換、エラー伝播の全体を抽出した証明ではありません。対応する Rust が変わった場合はモデルとの対応を再レビューします。
- `writer_eq_builder` が示すのは entries と children の列の一致です。文字列エスケープ、インデント、数値表記、I/O、メモリ使用量、実際の文字列全体の一致はここでは証明していません。
- newtype の payload は変換済みの値として扱います。内側の値変換が失敗するケースや、ユーザー実装の serializer/deserializer の正しさは対象外です。内側に入る前の variant 名の分岐と、内側の名前検査の有無をモデル化しています。
- Lean カーネルによる型検査を信頼します。空公理は数学的な仮定を追加していないという意味であり、モデルと Rust の対応そのものが自動証明されたという意味ではありません。
- Rust の公開 API、挙動、Cargo 依存は変更しません。既存の Rust テストと、実抽出した限定範囲の証明を併用します。
