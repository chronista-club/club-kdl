import ClubKdl

/- 実 Rust からの生成定義と、独立した期待仕様の一致。生成側は手修正しない。 -/
namespace ClubKdlProof

open ClubKdlGenerated.ast

def expectedAsInteger : Value → Option Aeneas.Std.I128
  | .Integer i => some i
  | _ => none

theorem as_integer_matches_spec (x : Value) :
    Value.impl.as_integer x = Aeneas.Std.Result.ok (expectedAsInteger x) := by
  cases x <;> rfl

#print axioms as_integer_matches_spec

def expectedAsFloat : Value → Option Aeneas.Std.F64
  | .Float f => some f
  | _ => none

theorem as_float_matches_spec (x : Value) :
    Value.impl.as_float x = Aeneas.Std.Result.ok (expectedAsFloat x) := by
  cases x <;> rfl

#print axioms as_float_matches_spec

def expectedAsBool : Value → Option Bool
  | .Bool b => some b
  | _ => none

theorem as_bool_matches_spec (x : Value) :
    Value.impl.as_bool x = Aeneas.Std.Result.ok (expectedAsBool x) := by
  cases x <;> rfl

#print axioms as_bool_matches_spec

def expectedIsNull : Value → Bool
  | .Null => true
  | _ => false

theorem is_null_matches_spec (x : Value) :
    Value.impl.is_null x = Aeneas.Std.Result.ok (expectedIsNull x) := by
  cases x <;> rfl

#print axioms is_null_matches_spec

def expectedEntryArg (v : Value) : Entry := ⟨none, none, v⟩

theorem entry_arg_matches_spec (v : Value) :
    Entry.impl.arg v = Aeneas.Std.Result.ok (expectedEntryArg v) := by
  rfl

#print axioms entry_arg_matches_spec

def expectedEntryValue (e : Entry) : Value := e.value

theorem entry_value_matches_spec (e : Entry) :
    Entry.impl.value e = Aeneas.Std.Result.ok (expectedEntryValue e) := by
  rfl

#print axioms entry_value_matches_spec

-- Vec の内容をそのまま Slice として返す（要素の順序・重複も含む）。
def expectedDocumentNodes (d : Document) : Aeneas.Std.Slice Node :=
  d.root.children.slice

theorem document_nodes_matches_spec (d : Document) :
    Document.impl.nodes d = Aeneas.Std.Result.ok (expectedDocumentNodes d) := by
  rfl

#print axioms document_nodes_matches_spec

def expectedDocumentAsNode (d : Document) : Node := d.root

theorem document_as_node_matches_spec (d : Document) :
    Document.impl.as_node d = Aeneas.Std.Result.ok (expectedDocumentAsNode d) := by
  cases d with
  | mk root => cases root; rfl

#print axioms document_as_node_matches_spec

end ClubKdlProof
