import Aeneas

/-
  Aeneas 2e3d448 emits F64 but its Lean library does not define it.
  This carrier only preserves 64 bits; no floating-point operations are modeled.
  NaNs, infinities, and signed zero are distinct bit patterns, not arithmetic values.
-/
namespace Aeneas.Std

abbrev F64 := BitVec 64

end Aeneas.Std

-- Aeneas method result types use unqualified Bool. Within Value.impl,
-- resolve it to the builtin type instead of the Value.Bool constructor.
abbrev ClubKdlGenerated.ast.Value.impl.Bool := _root_.Bool

-- Upstream Vec.deref reconstructs a Slice from its list via lemmas that use
-- Classical.choice. The Vec already stores the exact Slice, so this local
-- primitive model returns it directly. No generated Rust declaration changes.
def ClubKdlGenerated.alloc.vec.Vec.deref {T : Type} (v : Aeneas.Std.alloc.vec.Vec T) :
    Aeneas.Std.Slice T := v.slice
