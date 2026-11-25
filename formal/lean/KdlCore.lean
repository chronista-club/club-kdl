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


-- edit: active entries in depth-first order
inductive EditNode where
  | mk (entries : List Val) (children : List EditNode)

def EditNode.entries : EditNode → List Val | .mk es _ => es

mutual
  def dfsNode : EditNode → List Val
    | .mk es cs => es ++ dfsEntries cs
    termination_by structural n => n
  def dfsEntries : List EditNode → List Val
    | [] => []
    | n :: ns => dfsNode n ++ dfsEntries ns
    termination_by structural ns => ns
end
mutual
  def entry_count : EditNode → Nat
    | .mk es cs => es.length + entries_count cs
    termination_by structural n => n
  def entries_count : List EditNode → Nat
    | [] => 0
    | n :: ns => entry_count n + entries_count ns
    termination_by structural ns => ns
end

-- Each skipped sibling contributes its entire subtree; descending skips parent entries.
mutual
  def locate_in_node (n : EditNode) (path : List Nat) : Option (EditNode × Nat) :=
    match path with
    | [] => some (n, 0)
    | _ :: _ => match n with
      | .mk es cs => (locate_node cs path).map fun (child, off) => (child, es.length + off)
    termination_by structural n
  def locate_node (nodes : List EditNode) (path : List Nat) : Option (EditNode × Nat) :=
    match nodes, path with
    | [], _ => none
    | _, [] => none
    | n :: _, 0 :: path => locate_in_node n path
    | n :: ns, (i + 1) :: path =>
      (locate_node ns (i :: path)).map fun (child, off) => (child, entry_count n + off)
    termination_by structural nodes
end

private theorem lengthAppend {α : Type} (xs ys : List α) :
    (xs ++ ys).length = xs.length + ys.length := by
  induction xs with
  | nil => exact (Nat.zero_add _).symm
  | cons x xs ih => exact (congrArg Nat.succ ih).trans (Nat.succ_add _ _).symm

mutual
  private theorem dfs_node_count (n : EditNode) : (dfsNode n).length = entry_count n := by
    cases n with
    | mk es cs =>
      exact (lengthAppend es (dfsEntries cs)).trans
        (congrArg (Nat.add es.length) (dfs_count cs))
    termination_by structural n
  private theorem dfs_count (ns : List EditNode) : (dfsEntries ns).length = entries_count ns := by
    cases ns with
    | nil => rfl
    | cons n ns =>
      exact (lengthAppend (dfsNode n) (dfsEntries ns)).trans
        (congrTwo Nat.add (dfs_node_count n) (dfs_count ns))
    termination_by structural ns
end

private theorem nthAppendLeft {α : Type} (xs ys : List α) (i : Nat) (v : α)
    (h : nth xs i = some v) : nth (xs ++ ys) i = some v := by
  induction xs generalizing i with
  | nil => cases i <;> cases h
  | cons x xs ih =>
    cases i with
    | zero => exact h
    | succ i => exact ih i h

private theorem nthAppendRight {α : Type} (xs ys : List α) (i : Nat) :
    nth (xs ++ ys) (xs.length + i) = nth ys i := by
  induction xs with
  | nil => exact congrArg (nth ys) (Nat.zero_add i)
  | cons x xs ih =>
    rw [show (x :: xs).length = xs.length + 1 from rfl, Nat.succ_add]
    exact ih

-- A successful path and active-entry index select exactly the parser's DFS slot.
mutual
  private theorem locate_in_node_matches (n : EditNode) (path : List Nat)
      (node : EditNode) (offset idx : Nat) (v : Val)
      (located : locate_in_node n path = some (node, offset))
      (atEntry : nth node.entries idx = some v) :
      nth (dfsNode n) (offset + idx) = some v := by
    cases path with
    | nil =>
      rw [locate_in_node] at located
      cases located
      cases n with
      | mk es cs =>
        rw [Nat.zero_add]
        exact nthAppendLeft es (dfsEntries cs) idx v atEntry
    | cons j path =>
      cases n with
      | mk es cs =>
        change (locate_node cs (j :: path)).map _ = _ at located
        cases hc : locate_node cs (j :: path) with
        | none => rw [hc] at located; cases located
        | some result =>
          cases result with
          | mk child off =>
            rw [hc] at located
            cases located
            have ih := locate_node_matches_dfs_entry cs (j :: path) node off idx v hc atEntry
            change nth (es ++ dfsEntries cs) ((es.length + off) + idx) = _
            rw [Nat.add_assoc]
            exact (nthAppendRight es (dfsEntries cs) (off + idx)).trans ih
    termination_by structural n

  theorem locate_node_matches_dfs_entry (nodes : List EditNode) (path : List Nat)
      (node : EditNode) (offset idx : Nat) (v : Val)
      (located : locate_node nodes path = some (node, offset))
      (atEntry : nth node.entries idx = some v) :
      nth (dfsEntries nodes) (offset + idx) = some v := by
    cases nodes with
    | nil => cases located
    | cons n ns =>
      cases path with
      | nil => cases located
      | cons i path =>
        cases i with
        | zero =>
          exact nthAppendLeft (dfsNode n) (dfsEntries ns) _ v
            (locate_in_node_matches n path node offset idx v located atEntry)
        | succ i =>
          change (locate_node ns (i :: path)).map _ = _ at located
          cases hc : locate_node ns (i :: path) with
          | none => rw [hc] at located; cases located
          | some result =>
            cases result with
            | mk child off =>
              rw [hc] at located
              cases located
              have ih := locate_node_matches_dfs_entry ns (i :: path) node off idx v hc atEntry
              change nth (dfsNode n ++ dfsEntries ns) ((entry_count n + off) + idx) = _
              rw [Nat.add_assoc, ← dfs_node_count n]
              exact (nthAppendRight (dfsNode n) (dfsEntries ns) (off + idx)).trans ih
    termination_by structural nodes
end

#print axioms lengthAppend
#print axioms dfs_node_count
#print axioms dfs_count
#print axioms nthAppendLeft
#print axioms nthAppendRight
#print axioms locate_in_node_matches
#print axioms locate_node_matches_dfs_entry

-- compact: recursive dictionary remapping

-- Annotations are passthrough payloads; only node/property names use the dictionary.
structure RemapEntry where
  name : Option String
  ty : Option String
  value : Val

inductive RemapNode where
  | mk (name : String) (ty : Option String) (entries : List RemapEntry)
      (children : List RemapNode)

def remapEntry (mapName : String → Option String) (e : RemapEntry) : Option RemapEntry :=
  match e.name with
  | none => some e
  | some name => (mapName name).map fun mapped => { e with name := some mapped }

def remapEntries (f : String → Option String) : List RemapEntry → Option (List RemapEntry)
  | [] => some []
  | e :: es => match remapEntry f e with
    | none => none
    | some e' => match remapEntries f es with
      | none => none
      | some es' => some (e' :: es')
  termination_by structural es => es

mutual
  def remapNode (f : String → Option String) : RemapNode → Option RemapNode
    | .mk name ty es cs => match f name with
      | none => none
      | some name' => match remapEntries f es with
        | none => none
        | some es' => match remapNodes f cs with
          | none => none
          | some cs' => some (.mk name' ty es' cs')
    termination_by structural n => n
  def remapNodes (f : String → Option String) : List RemapNode → Option (List RemapNode)
    | [] => some []
    | n :: ns => match remapNode f n with
      | none => none
      | some n' => match remapNodes f ns with
        | none => none
        | some ns' => some (n' :: ns')
    termination_by structural ns => ns
end

def InverseNames (encode decode : String → Option String) : Prop :=
  ∀ s a, encode s = some a → decode a = some s

theorem remap_argument_unchanged (f : String → Option String) (ty : Option String) (v : Val) :
    remapEntry f ⟨none, ty, v⟩ = some ⟨none, ty, v⟩ := by rfl
#print axioms remap_argument_unchanged

theorem remap_entry_roundtrip (encode decode : String → Option String)
    (inverse : InverseNames encode decode) (e e' : RemapEntry)
    (encoded : remapEntry encode e = some e') : remapEntry decode e' = some e := by
  cases e with
  | mk name ty value =>
    cases name with
    | none => cases encoded; rfl
    | some name =>
      change (encode name).map _ = _ at encoded
      cases hn : encode name with
      | none => rw [hn] at encoded; cases encoded
      | some alias =>
        rw [hn] at encoded
        cases encoded
        change (decode alias).map _ = _
        rw [inverse name alias hn]
        rfl
#print axioms remap_entry_roundtrip

theorem remap_entries_roundtrip (encode decode : String → Option String)
    (inverse : InverseNames encode decode) (es es' : List RemapEntry)
    (encoded : remapEntries encode es = some es') : remapEntries decode es' = some es := by
  induction es generalizing es' with
  | nil => cases encoded; rfl
  | cons e es ih =>
    unfold remapEntries at encoded
    cases he : remapEntry encode e with
    | none => rw [he] at encoded; cases encoded
    | some e' =>
      rw [he] at encoded
      cases ht : remapEntries encode es with
      | none => rw [ht] at encoded; cases encoded
      | some tail =>
        rw [ht] at encoded
        cases encoded
        unfold remapEntries
        rw [remap_entry_roundtrip encode decode inverse e e' he, ih tail ht]
#print axioms remap_entries_roundtrip

mutual
  theorem remap_node_roundtrip (encode decode : String → Option String)
      (inverse : InverseNames encode decode) (n n' : RemapNode)
      (encoded : remapNode encode n = some n') : remapNode decode n' = some n := by
    cases n with
    | mk name ty es cs =>
      unfold remapNode at encoded
      cases hn : encode name with
      | none => rw [hn] at encoded; cases encoded
      | some alias =>
        rw [hn] at encoded
        cases he : remapEntries encode es with
        | none => rw [he] at encoded; cases encoded
        | some es' =>
          rw [he] at encoded
          cases hc : remapNodes encode cs with
          | none => rw [hc] at encoded; cases encoded
          | some cs' =>
            rw [hc] at encoded
            cases encoded
            unfold remapNode
            rw [inverse name alias hn, remap_entries_roundtrip encode decode inverse es es' he,
              remap_nodes_roundtrip encode decode inverse cs cs' hc]
    termination_by structural n
  theorem remap_nodes_roundtrip (encode decode : String → Option String)
      (inverse : InverseNames encode decode) (ns ns' : List RemapNode)
      (encoded : remapNodes encode ns = some ns') : remapNodes decode ns' = some ns := by
    cases ns with
    | nil => cases encoded; rfl
    | cons n ns =>
      unfold remapNodes at encoded
      cases hn : remapNode encode n with
      | none => rw [hn] at encoded; cases encoded
      | some n' =>
        rw [hn] at encoded
        cases ht : remapNodes encode ns with
        | none => rw [ht] at encoded; cases encoded
        | some tail =>
          rw [ht] at encoded
          cases encoded
          unfold remapNodes
          rw [remap_node_roundtrip encode decode inverse n n' hn,
            remap_nodes_roundtrip encode decode inverse ns tail ht]
    termination_by structural ns
end
#print axioms remap_node_roundtrip
#print axioms remap_nodes_roundtrip

-- One alias cannot decode back to two distinct original names.
theorem shadowing_breaks_roundtrip (encode decode : String → Option String)
    (s1 s2 alias : String) (different : s1 ≠ s2)
    (e1 : encode s1 = some alias) (e2 : encode s2 = some alias) :
    (encode s1).bind decode ≠ some s1 ∨ (encode s2).bind decode ≠ some s2 := by
  rw [e1, e2]
  change decode alias ≠ some s1 ∨ decode alias ≠ some s2
  cases decode alias with
  | none => exact Or.inl (fun h => nomatch h)
  | some s =>
    cases decEq s s1 with
    | isFalse h => exact Or.inl (fun eq => h (Option.some.inj eq))
    | isTrue h =>
      exact Or.inr (fun eq => different (h.symm.trans (Option.some.inj eq)))
#print axioms shadowing_breaks_roundtrip

-- compose: filter, rename, then prefix

-- Character lists model text constructively: Lean's optimized String.append / BEq
-- depend on axioms even when a theorem never evaluates them.
abbrev ComposeName := List Char
inductive ComposeVal where
  | null
  | integer (value : Int)
  | floatBits (value : BitVec 64)
  | bool (value : Bool)
  | string (value : ComposeName)
structure ComposeEntry where
  name : Option ComposeName
  ty : Option ComposeName
  value : ComposeVal
inductive ComposeNode where
  | mk (name : ComposeName) (ty : Option ComposeName) (entries : List ComposeEntry)
      (children : List ComposeNode)

def containsName : List ComposeName → ComposeName → Bool
  | [], _ => false
  | n :: ns, name => if n = name then true else containsName ns name

structure Directive where
  only : Option (List ComposeName)
  except : List ComposeName
  rename : ComposeName → Option ComposeName
  as_prefix : Option ComposeName

-- arg(0).and_then(as_string): a non-string first positional arg stops the search.
def firstEntryString : List ComposeEntry → Option ComposeName
  | [] => none
  | e :: es => match e.name with
    | some _ => firstEntryString es
    | none => match e.value with
      | .string s => some s
      | .null => none
      | .integer _ => none
      | .floatBits _ => none
      | .bool _ => none
  termination_by structural es => es

def firstStringArg : ComposeNode → Option ComposeName
  | .mk _ _ es _ => firstEntryString es

-- Rust creates Entry::arg, so the replaced string's annotation is cleared.
def setFirstEntryString (name : ComposeName) : List ComposeEntry → List ComposeEntry
  | [] => []
  | e :: es => match e.name with
    | some _ => e :: setFirstEntryString name es
    | none => match e.value with
      | .string _ => ⟨none, none, .string name⟩ :: es
      | .null => e :: setFirstEntryString name es
      | .integer _ => e :: setFirstEntryString name es
      | .floatBits _ => e :: setFirstEntryString name es
      | .bool _ => e :: setFirstEntryString name es
  termination_by structural es => es

def setFirstStringArg (n : ComposeNode) (name : ComposeName) : ComposeNode :=
  match n with
  | .mk kind ty es cs => .mk kind ty (setFirstEntryString name es) cs

def onlyAllows (d : Directive) (original : ComposeName) : Bool :=
  match d.only with
  | none => true
  | some names => containsName names original

def renamed (d : Directive) (original : ComposeName) : ComposeName :=
  match d.rename original with
  | none => original
  | some name => name

def prefixed (d : Directive) (name : ComposeName) : ComposeName :=
  match d.as_prefix with
  | none => name
  | some pfx => pfx ++ ['.'] ++ name

def transform_node (d : Directive) (n : ComposeNode) : Option ComposeNode :=
  match firstStringArg n with
  | none => some n
  | some original => match onlyAllows d original with
    | false => none
    | true => match containsName d.except original with
      | true => none
      | false => some (setFirstStringArg n (prefixed d (renamed d original)))

private theorem setFirstEntryString_matches (es : List ComposeEntry) (original name : ComposeName)
    (named : firstEntryString es = some original) :
    firstEntryString (setFirstEntryString name es) = some name := by
  induction es with
  | nil => cases named
  | cons e es ih =>
    cases e with
    | mk key ty value =>
      cases key with
      | some k => exact ih named
      | none =>
        cases value with
        | string s => rfl
        | null => cases named
        | integer _ => cases named
        | floatBits _ => cases named
        | bool _ => cases named

private theorem setFirstStringArg_matches (n : ComposeNode) (original name : ComposeName)
    (named : firstStringArg n = some original) :
    firstStringArg (setFirstStringArg n name) = some name := by
  cases n with
  | mk kind ty es cs => exact setFirstEntryString_matches es original name named

-- No first positional string: filters, rename and prefix have no effect at all.
theorem transform_preserves_nameless (d : Directive) (n : ComposeNode)
    (nameless : firstStringArg n = none) : transform_node d n = some n := by
  unfold transform_node
  rw [nameless]
#print axioms transform_preserves_nameless

-- There is deliberately no membership constraint on the renamed/prefixed name.
theorem transform_filters_on_original_name (d : Directive) (n : ComposeNode) (original : ComposeName)
    (named : firstStringArg n = some original)
    (included : onlyAllows d original = true) (notExcluded : containsName d.except original = false) :
    transform_node d n = some (setFirstStringArg n (prefixed d (renamed d original))) ∧
    firstStringArg (setFirstStringArg n (prefixed d (renamed d original))) =
      some (prefixed d (renamed d original)) := by
  constructor
  · unfold transform_node
    rw [named]
    dsimp only
    rw [included]
    dsimp only
    rw [notExcluded]
  · exact setFirstStringArg_matches n original _ named
#print axioms transform_filters_on_original_name

-- If filters ran after rename, a new name rejected by only/except would be lost.
theorem rename_filter_noncommute (d : Directive) (n : ComposeNode) (original : ComposeName)
    (named : firstStringArg n = some original)
    (included : onlyAllows d original = true) (notExcluded : containsName d.except original = false)
    (renamedRejected : onlyAllows d (renamed d original) = false ∨
      containsName d.except (renamed d original) = true) :
    transform_node d n = some (setFirstStringArg n (prefixed d (renamed d original))) ∧
    transform_node d (setFirstStringArg n (renamed d original)) = none := by
  constructor
  · exact (transform_filters_on_original_name d n original named included notExcluded).1
  · unfold transform_node
    rw [setFirstStringArg_matches n original _ named]
    dsimp only
    cases renamedRejected with
    | inl h => rw [h]
    | inr h =>
      rw [h]
      cases onlyAllows d (renamed d original) <;> rfl
#print axioms rename_filter_noncommute
#print axioms setFirstEntryString_matches
#print axioms setFirstStringArg_matches

end KdlCore
