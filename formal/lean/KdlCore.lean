/-
  club-kdl の構造法則。手書きモデルであり、Rust 抽出証明は ../aeneas を参照。
  標準 List 補題の中には propext を使うものがあるため、必要な補題も構成的に示す。
-/
namespace KdlCore

inductive Val where
  | null
  | integer (value : Int)
  | floatBits (value : BitVec 64)
  | bool (value : Bool)
  | string (value : String)

structure Entry where
  name : Option String
  value : Val

def Entry.arg (v : Val) : Entry := ⟨none, v⟩
def Entry.prop (k : String) (v : Val) : Entry := ⟨some k, v⟩

def args (es : List Entry) : List Val :=
  es.filterMap fun e => match e.name with | none => some e.value | some _ => none

-- Rust Iterator::nth の構造モデル（Lean の境界判定の補題へ依存しない）。
def nth {α : Type} : List α → Nat → Option α
  | [], _ => none
  | x :: _, 0 => some x
  | _ :: xs, i + 1 => nth xs i

def arg (es : List Entry) (i : Nat) : Option Val := nth (args es) i

def hasKey (k : String) (e : Entry) : Bool :=
  match e.name with | none => false | some name => decide (name = k)

-- 実 ast::Node::prop と同じ reverse + find? による右端優先。
def prop (es : List Entry) (k : String) : Option Val :=
  (es.reverse.find? (hasKey k)).map Entry.value

-- 公理を持ち込まない補題。
private theorem congrTwo {α β γ : Type} (f : α → β → γ) {a a' : α} {b b' : β}
    (ha : a = a') (hb : b = b') : f a b = f a' b' := by
  cases ha
  cases hb
  rfl

private theorem appendAssoc {α : Type} (a b c : List α) :
    (a ++ b) ++ c = a ++ (b ++ c) := by
  induction a with
  | nil => rfl
  | cons x xs ih => exact congrArg (List.cons x) ih

private theorem appendNil {α : Type} (a : List α) : a ++ [] = a := by
  induction a with
  | nil => rfl
  | cons x xs ih => exact congrArg (List.cons x) ih

private theorem reverseAcc {α : Type} (xs acc : List α) :
    xs.reverseAux acc = xs.reverse ++ acc := by
  have aux (ys a b : List α) : ys.reverseAux (a ++ b) = ys.reverseAux a ++ b := by
    induction ys generalizing a with
    | nil => rfl
    | cons y ys ih => exact ih (y :: a)
  exact aux xs [] acc

private theorem reverseAppend {α : Type} (xs ys : List α) :
    (xs ++ ys).reverse = ys.reverse ++ xs.reverse := by
  have aux (a b acc : List α) :
      (a ++ b).reverseAux acc = b.reverseAux (a.reverseAux acc) := by
    induction a generalizing acc with
    | nil => rfl
    | cons x a ih => exact ih (x :: acc)
  exact (aux xs ys []).trans (reverseAcc ys xs.reverse)

private theorem findSkip {α : Type} (p : α → Bool) (a b : List α) (x : α)
    (h : p x = false) : (a ++ x :: b).find? p = (a ++ b).find? p := by
  induction a with
  | nil => change (match p x with | true => some x | false => b.find? p) = b.find? p
           rw [h]
  | cons y a ih =>
    change (match p y with | true => some y | false => (a ++ x :: b).find? p) =
      (match p y with | true => some y | false => (a ++ b).find? p)
    cases p y with
    | false => exact ih
    | true => rfl

-- 任意位置。先頭と末尾は a=[] / b=[] の特殊例。
theorem args_insert_prop (a b : List Entry) (k : String) (v : Val) :
    args (a ++ Entry.prop k v :: b) = args (a ++ b) := by
  induction a with
  | nil => rfl
  | cons e a ih =>
    cases e with
    | mk name value =>
      cases name with
      | none => exact congrArg (List.cons value) ih
      | some _ => exact ih

#print axioms args_insert_prop

theorem arg_insert_prop (a b : List Entry) (k : String) (v : Val) (i : Nat) :
    arg (a ++ Entry.prop k v :: b) i = arg (a ++ b) i := by
  exact congrArg (fun vs : List Val => nth vs i) (args_insert_prop a b k v)

#print axioms arg_insert_prop

theorem args_append_prop (es : List Entry) (k : String) (v : Val) :
    args (es ++ [Entry.prop k v]) = args es := by
  exact (args_insert_prop es [] k v).trans (congrArg args (appendNil es))

#print axioms args_append_prop

theorem arg_append_prop (es : List Entry) (k : String) (v : Val) (i : Nat) :
    arg (es ++ [Entry.prop k v]) i = arg es i := by
  exact congrArg (fun vs : List Val => nth vs i) (args_append_prop es k v)

#print axioms arg_append_prop

theorem prop_insert_arg (a b : List Entry) (v : Val) (k : String) :
    prop (a ++ Entry.arg v :: b) k = prop (a ++ b) k := by
  unfold prop
  rw [reverseAppend, reverseAppend]
  have hrev : (Entry.arg v :: b).reverse = b.reverse ++ [Entry.arg v] :=
    reverseAcc b [Entry.arg v]
  rw [hrev, appendAssoc]
  exact congrArg (Option.map Entry.value) (findSkip (hasKey k) b.reverse a.reverse (Entry.arg v) rfl)

#print axioms prop_insert_arg

theorem prop_append_arg (es : List Entry) (v : Val) (k : String) :
    prop (es ++ [Entry.arg v]) k = prop es k := by
  exact (prop_insert_arg es [] v k).trans (congrArg (fun xs => prop xs k) (appendNil es))

#print axioms prop_append_arg

theorem prop_append_same (es : List Entry) (k : String) (v : Val) :
    prop (es ++ [Entry.prop k v]) k = some v := by
  unfold prop
  rw [reverseAppend]
  change (match decide (k = k) with
    | true => some (Entry.prop k v)
    | false => es.reverse.find? (hasKey k)).map Entry.value = some v
  rw [show decide (k = k) = true from decide_eq_true rfl]
  rfl

#print axioms prop_append_same

theorem prop_append_ne (es : List Entry) (k other : String) (v : Val) (h : k ≠ other) :
    prop (es ++ [Entry.prop k v]) other = prop es other := by
  unfold prop
  rw [reverseAppend]
  change (match decide (k = other) with
    | true => some (Entry.prop k v)
    | false => es.reverse.find? (hasKey other)).map Entry.value = _
  rw [show decide (k = other) = false from decide_eq_false h]

#print axioms prop_append_ne

-- Option: 位置を保つ null と、プロパティそのものの省略は別操作。
def isNull : Val → Bool
  | .null => true
  | .integer _ => false
  | .floatBits _ => false
  | .bool _ => false
  | .string _ => false

def encodeArg : Option Val → Val | none => .null | some v => v

-- filter (not isNull) の分岐を明記し、ライブラリ補題への公理依存を避ける。
def readOption : Option Val → Option Val
  | none => none
  | some v => match isNull v with | true => none | false => some v

def NonNull : Option Val → Prop | none => True | some v => v ≠ .null

theorem option_arg_roundtrip (opt : Option Val) (h : NonNull opt) :
    readOption (some (encodeArg opt)) = opt := by
  cases opt with
  | none => rfl
  | some v =>
    cases v with
    | null => exact False.elim (h rfl)
    | integer _ => rfl
    | floatBits _ => rfl
    | bool _ => rfl
    | string _ => rfl

#print axioms option_arg_roundtrip

-- 全スロットを出力し、一つの none も消さない。
def encodeArgs (xs : List (Option Val)) : List Val := xs.map encodeArg

private theorem mapIndex {α β : Type} (f : α → β) (xs : List α) (i : Nat) :
    nth (xs.map f) i = (nth xs i).map f := by
  induction xs generalizing i with
  | nil => cases i <;> rfl
  | cons x xs ih =>
    cases i with
    | zero => rfl
    | succ i => exact ih i

theorem option_args_indices (xs : List (Option Val)) (i : Nat) :
    nth (encodeArgs xs) i = (nth xs i).map encodeArg := mapIndex encodeArg xs i

#print axioms option_args_indices

theorem option_args_roundtrip_at (xs : List (Option Val)) (i : Nat) (opt : Option Val)
    (atIndex : nth xs i = some opt) (h : NonNull opt) :
    readOption (nth (encodeArgs xs) i) = opt := by
  rw [option_args_indices, atIndex]
  exact option_arg_roundtrip opt h

#print axioms option_args_roundtrip_at

theorem option_args_length (xs : List (Option Val)) : (encodeArgs xs).length = xs.length := by
  induction xs with
  | nil => rfl
  | cons _ xs ih => exact congrArg Nat.succ ih

#print axioms option_args_length

-- none を省略する誤った実装に対する一般的な反例。
def badOmitArgs (xs : List (Option Val)) : List Val := xs.filterMap id

theorem omitting_none_shifts_index (v : Val) (h : v ≠ .null) :
    readOption (nth (badOmitArgs [none, some v]) 0) = some v ∧
    readOption (nth (badOmitArgs [none, some v]) 1) = none ∧
    readOption (nth (badOmitArgs [none, some v]) 0) ≠ none := by
  have first : readOption (some v) = some v := option_arg_roundtrip (some v) h
  refine ⟨first, rfl, ?_⟩
  intro impossible
  have contradiction : (some v : Option Val) = none := first.symm.trans impossible
  cases contradiction

#print axioms omitting_none_shifts_index

def encodeProp (k : String) : Option Val → List Entry
  | none => []
  | some v => [Entry.prop k v]

theorem option_prop_roundtrip (k : String) (opt : Option Val) (h : NonNull opt) :
    readOption (prop (encodeProp k opt) k) = opt := by
  cases opt with
  | none => rfl
  | some v =>
    have hp : prop (encodeProp k (some v)) k = some v := prop_append_same [] k v
    rw [hp]
    exact option_arg_roundtrip (some v) h

#print axioms option_prop_roundtrip

-- 任意ネストの flatten。flatten は左の構造体へ右の構造体を取り込む。
-- Child は既に生成した子ノードの不透明な値: 名前、値、子孫などをそのまま保つ。
inductive FlattenTree (Child : Type) where
  | fields (entries : List Entry) (children : List Child)
  | flatten (parent nested : FlattenTree Child)

def builder {C : Type} : FlattenTree C → List Entry × List C
  | .fields es cs => (es, cs)
  | .flatten parent nested =>
    let p := builder parent
    let n := builder nested
    (p.1 ++ n.1, p.2 ++ n.2)

-- writer の二つの独立した再帰走査（全 entries を流してから全 children）。
def writeEntries {C : Type} : FlattenTree C → List Entry
  | .fields es _ => es
  | .flatten parent nested => writeEntries parent ++ writeEntries nested

def writeChildren {C : Type} : FlattenTree C → List C
  | .fields _ cs => cs
  | .flatten parent nested => writeChildren parent ++ writeChildren nested

def writer {C : Type} (tree : FlattenTree C) : List Entry × List C :=
  (writeEntries tree, writeChildren tree)

theorem writer_eq_builder {C : Type} (tree : FlattenTree C) : writer tree = builder tree := by
  induction tree with
  | fields _ _ => rfl
  | flatten p n ihp ihn =>
    have entriesP := congrArg Prod.fst ihp
    have entriesN := congrArg Prod.fst ihn
    have childrenP := congrArg Prod.snd ihp
    have childrenN := congrArg Prod.snd ihn
    exact congrTwo Prod.mk (congrTwo List.append entriesP entriesN)
      (congrTwo List.append childrenP childrenN)

#print axioms writer_eq_builder

-- newtype enum: 名前を variant に上書きした後、内側の名前検査をスキップする。
structure NamedNode (Payload : Type) where
  name : String
  payload : Payload

def serializeInner {P : Type} (innerName : String) (value : P) : NamedNode P :=
  ⟨innerName, value⟩

def serializeVariant {P : Type} (variantName innerName : String) (value : P) : NamedNode P :=
  { serializeInner innerName value with name := variantName }

def fromUnnamed {P : Type} (node : NamedNode P) : Option P := some node.payload

def fromNamed {P : Type} (expectedName : String) (node : NamedNode P) : Option P :=
  if node.name = expectedName then fromUnnamed node else none

def deserializeVariant {P : Type} (variantName : String) (node : NamedNode P) : Option P :=
  if node.name = variantName then fromUnnamed node else none

theorem newtype_unnamed_roundtrip {P : Type} (variantName innerName : String) (value : P) :
    deserializeVariant variantName (serializeVariant variantName innerName value) = some value := by
  change (if variantName = variantName then some value else none) = some value
  exact ite_eq_left rfl

#print axioms newtype_unnamed_roundtrip

theorem newtype_named_fails {P : Type} (variantName innerName : String) (value : P)
    (h : variantName ≠ innerName) :
    fromNamed innerName (serializeVariant variantName innerName value) = none := by
  change (if variantName = innerName then some value else none) = none
  exact ite_eq_right h

#print axioms newtype_named_fails

-- 補助定理も個別に監査する。
#print axioms congrTwo
#print axioms appendAssoc
#print axioms appendNil
#print axioms reverseAcc
#print axioms reverseAppend
#print axioms findSkip
#print axioms mapIndex

end KdlCore
