// mem_1ChPFWRE796pYMFmCiUuku
// club-kdl の有限抽象モデル。Rust の証明ではなく設計規則の反例探索。
// Node は親 + 葉の 2 階層、flatten は親へ展開する 2 段の部品列で表す。
module deriveModel {
  const NULL_FOR_NONE_ARG: bool
  const UNNAMED_NEWTYPE_ENUM: bool
  // 検査フラグではなく、シリアライザと読取 index が整合するスキーマの前提。
  const STRICT_ARG_ORDER: bool
  const QUOTE_AMBIGUOUS_IDENT: bool

  type Value = Text(str) | Null | Integer(int) | Float(str) | BooleanValue(bool) | Invalid
  type Entry = { key: str, value: Value } // key="" は位置引数（空の property 名は対象外）
  type Leaf = { name: str, entries: List[Entry] }
  type Node = { name: str, entries: List[Entry], children: List[Leaf] }
  type Arg = { value: Value, index: int } // -1 は自動採番
  type Prop = { key: str, value: Value, optional: bool }
  type Part = { args: List[Value], props: List[Prop], children: List[Leaf] }
  type Input = {
    args: List[Arg], rest: List[Value], props: List[Prop], children: List[Leaf], flattens: List[Part]
  }
  type Variant = { tag: str, payload: str }
  type Lexeme = { text: str, chars: List[str] }
  type Token = { lexeme: Lexeme, quoted: bool }

  pure def arg(v: Value): Entry = { key: "", value: v }
  pure def prop(k: str, v: Value): Entry = { key: k, value: v }
  pure def leaf(n: str, s: str): Leaf = { name: n, entries: List(arg(Text(s))) }
  pure def argsOf(es: List[Entry]): List[Value] = es.select(e => e.key == "").foldl(List(), (out, e) => out.append(e.value))
  pure def at(xs: List[Value], i: int): Value = if (i < xs.length()) xs[i] else Null
  // Node::prop の rev().find() と等価な左からの上書き。重複を消さずに保持する。
  pure def lookup(es: List[Entry], k: str): Value =
    es.foldl(Null, (v, e) => if (e.key == k) e.value else v)
  pure def required(v: Value): Value = if (v == Null) Invalid else v
  pure def defaulted(xs: List[Value], i: int): Value =
    if (i < xs.length()) xs[i] else Text("")

  // 明示 index を昇順、残りは宣言順。有限スキーマで使う index は 0..3。
  pure def ordered(xs: List[Arg]): List[Arg] =
    List(0, 1, 2, 3, -1).foldl(List(), (acc, i) => acc.concat(xs.select(a => a.index == i)))
  pure def emitArgs(xs: List[Arg]): List[Entry] = ordered(xs)
    .select(a => NULL_FOR_NONE_ARG or a.value != Null).foldl(List(), (out, a) => out.append(arg(a.value)))
  pure def emitProps(ps: List[Prop]): List[Entry] =
    ps.select(p => not(p.optional) or p.value != Null).foldl(List(), (out, p) => out.append(prop(p.key, p.value)))

  // NodeBuilder: 部品ごとの AST を作り、flatten で entries と children を別々に追加。
  pure def serializeTree(x: Input): Node = {
    val base = { name: "config", entries: emitArgs(x.args).concat(x.rest.foldl(List(), (es, v) => es.append(arg(v)))).concat(emitProps(x.props)), children: x.children }
    x.flattens.foldl(base, (n, f) => {
      val built = { name: "ignored", entries: f.args.foldl(List(), (out, v) => out.append(arg(v))).concat(emitProps(f.props)), children: f.children }
      { ...n, entries: n.entries.concat(built.entries), children: n.children.concat(built.children) }
    })
  }

  // NodeWriter: AST の flatten を使わず、__write_entries と __write_children の二巡。
  // テキストの空白は捨象し、出力イベントを正規化したノード構造を比較する。
  pure def serializeDirect(x: Input): Node = {
    val arguments = ordered(x.args).foldl(List(), (out, a) =>
      if (a.value == Null and not(NULL_FOR_NONE_ARG)) out else out.append(arg(a.value)))
    val multi = x.rest.foldl(arguments, (out, v) => out.append(arg(v)))
    val ownEntries = x.props.foldl(multi, (out, p) =>
      if (p.optional and p.value == Null) out else out.append(prop(p.key, p.value)))
    val entryPass = x.flattens.foldl(ownEntries, (out, f) => {
      val flatArgs = f.args.foldl(out, (es, v) => es.append(arg(v)))
      f.props.foldl(flatArgs, (es, p) =>
        if (p.optional and p.value == Null) es else es.append(prop(p.key, p.value)))
    })
    val childPass = x.flattens.foldl(x.children, (out, f) =>
      f.children.foldl(out, (cs, c) => cs.append(c)))
    { name: "config", entries: entryPass, children: childPass }
  }

  // 明示 index は自動カウンタを進めない (derive/src/lib.rs generate_field_deserializers)。
  pure def readArgs(x: Input, n: Node): List[Value] = {
    val result = x.args.foldl({ next: 0, values: List() }, (r, a) => {
      val i = if (a.index == -1) r.next else a.index
      { next: if (a.index == -1) r.next + 1 else r.next,
        values: r.values.append(at(argsOf(n.entries), i)) }
    })
    result.values
  }

  pure def sample(a: Value, b: Value, p: Value, c: bool, nested: bool): Input = {
    // 整合例 = 全自動。非整合例 = 先頭が explicit(1)、次が auto(0)。
    val as = List({ value: a, index: if (STRICT_ARG_ORDER) -1 else 1 }, { value: b, index: -1 })
    { args: as, rest: List(),
      props: List({ key: "renamed", value: Text("required"), optional: false },
                  { key: "optional", value: p, optional: true }),
      children: if (c) List(leaf("inner", "child"), leaf("unwrapped", "scalar"), leaf("item", "one"), leaf("item", "two")) else List(),
      flattens: if (nested) List(
        { args: List(), props: List({ key: "flat", value: Text("outer"), optional: false }), children: List(leaf("flat-child", "outer")) },
        { args: List(), props: List({ key: "deep", value: Text("inner"), optional: false }), children: List(leaf("deep-child", "inner")) }
      ) else List() }
  }

  // Data Enum: newtype の外側は variant 名、内側 struct の名前は inner。
  pure def writeEnum(v: Variant): Leaf =
    if (v.tag == "idle") { name: "idle", entries: List() }
    else if (v.tag == "named") { name: "named", entries: List(prop("renamed", Text(v.payload))) }
    else leaf("wrapped", v.payload)
  pure def readEnum(n: Leaf): Variant =
    if (n.name == "idle") { tag: "idle", payload: "" }
    else if (n.name == "named") { tag: "named", payload: textOf(required(lookup(n.entries, "renamed"))) }
    else if (n.name == "wrapped" and UNNAMED_NEWTYPE_ENUM)
      { tag: "wrapped", payload: textOf(required(at(argsOf(n.entries), 0))) }
    else { tag: "UnexpectedNode", payload: "" }
  pure def textOf(v: Value): str = match v { Text(s) => s | _ => "ERROR" }
  pure def readChildren(cs: List[Leaf], matchesAny: bool): List[Leaf] =
    if (matchesAny) cs else cs.select(c => c.name == "item")
  pure def childArg(cs: List[Leaf], name: str): Value = {
    val found = cs.select(c => c.name == name)
    if (found.length() == 0) Null else at(argsOf(found[0].entries), 0)
  }

  pure def writeUnwrapArgs(name: str, values: List[Value]): Leaf =
    { name: name, entries: values.foldl(List(), (es, v) => es.append(arg(v))) }
  pure def readUnwrapArgs(cs: List[Leaf], name: str): List[Value] = {
    val found = cs.select(c => c.name == name)
    if (found.length() == 0) List() else argsOf(found[0].entries)
  }

  // 同じ親ノードから flatten のプロパティ・子も読み戻す（名前無し struct の範囲）。
  pure def readStruct(x, n) = {
    args: readArgs(x, n), requiredProp: required(lookup(n.entries, "renamed")),
    optionalProp: lookup(n.entries, "optional"), child: childArg(n.children, "inner"),
    unwrapped: childArg(n.children, "unwrapped"),
    items: readChildren(n.children, false).foldl(List(), (vs, c) => vs.append(at(argsOf(c.entries), 0))),
    flat: lookup(n.entries, "flat"), deep: lookup(n.entries, "deep"),
    flatChild: childArg(n.children, "flat-child"), deepChild: childArg(n.children, "deep-child"),
  }

  // 文字列演算がない Quint では、文字列とその文字列を表す文字リストを対にする。
  // 候補の対応をシナリオで固定。parse は候補領域での実 parser の結果を独立に表す。
  pure val CANDIDATES = Set(
    { text: "api", chars: List("a", "p", "i") }, { text: "", chars: List() },
    { text: "true", chars: List("t", "r", "u", "e") }, { text: "false", chars: List("f", "a", "l", "s", "e") },
    { text: "null", chars: List("n", "u", "l", "l") }, { text: "inf", chars: List("i", "n", "f") },
    { text: "-inf", chars: List("-", "i", "n", "f") }, { text: "nan", chars: List("n", "a", "n") },
    { text: "1", chars: List("1") }, { text: "-1", chars: List("-", "1") }, { text: "+1", chars: List("+", "1") },
    { text: "1.5", chars: List("1", ".", "5") }, { text: ".5", chars: List(".", "5") },
    { text: "-.5", chars: List("-", ".", "5") }, { text: "+.5", chars: List("+", ".", "5") },
    { text: "-.x", chars: List("-", ".", "x") }, { text: "+", chars: List("+") }, { text: ".", chars: List(".") },
    { text: "a b", chars: List("a", " ", "b") }, { text: "a=b", chars: List("a", "=", "b") },
    { text: "日本", chars: List("日", "本") }
  )
  pure val DIGITS = Set("0", "1", "2", "3", "4", "5", "6", "7", "8", "9")
  pure val RESERVED = Set("true", "false", "null", "inf", "-inf", "nan")
  pure def charAt(cs: List[str], i: int): str = if (i < cs.length()) cs[i] else ""
  pure def isBare(l: Lexeme): bool = {
    val first = charAt(l.chars, 0)
    val sign = if (Set("+", "-").contains(first)) 1 else 0
    val numeric = DIGITS.contains(charAt(l.chars, sign)) or
      (charAt(l.chars, sign) == "." and DIGITS.contains(charAt(l.chars, sign + 1)))
    and {
      l.chars.length() > 0,
      not(DIGITS.contains(first)),
      l.chars.foldl(true, (ok, c) => ok and not(Set(" ", "=", "{", "}", ";", "#").contains(c))),
      not(QUOTE_AMBIGUOUS_IDENT) or (not(numeric) and not(RESERVED.contains(l.text))),
    }
  }
  pure def writeString(l: Lexeme): Token = { lexeme: l, quoted: not(isBare(l)) }
  pure def parseValueToken(t: Token): Value = {
    val s = t.lexeme.text
    if (t.quoted) Text(s)
    else if (Set("true", "false", "null", "inf", "-inf", "nan", ".5", "-.5", "+.5", "", "a b", "a=b").contains(s)) Invalid
    else if (s == "-1") Integer(-1)
    else if (Set("1", "+1").contains(s)) Integer(1)
    else if (s == "1.5") Float("1.5")
    else if (s == "#true") BooleanValue(true)
    else if (s == "#false") BooleanValue(false)
    else if (s == "#null") Null
    else Text(s)
  }

  var input: Input
  var enumValue: Variant
  var string: Lexeme
  var extra: Value
  action setCase(x: Input, v: Variant, s: Lexeme, e: Value): bool = all {
    input' = x, enumValue' = v, string' = s, extra' = e,
  }
  pure val API = { text: "api", chars: List("a", "p", "i") }
  action init = setCase(sample(Text("a"), Text("a"), Null, false, false), { tag: "idle", payload: "" }, API, Text("first"))
  action step = {
    nondet a = Set(Null, Text("a"), Text("b")).oneOf()
    nondet b = Set(Null, Text("a"), Text("b")).oneOf()
    nondet p = Set(Null, Text("p")).oneOf()
    nondet c = Set(false, true).oneOf()
    nondet f = Set(false, true).oneOf()
    nondet v = Set({ tag: "idle", payload: "" }, { tag: "named", payload: "x" }, { tag: "wrapped", payload: "y" }).oneOf()
    nondet s = CANDIDATES.oneOf()
    nondet e = Set(Null, Text("last"), Integer(2)).oneOf()
    setCase(sample(a, b, p, c, f), v, s, e)
  }

  val writerMatchesBuilder = serializeTree(input) == serializeDirect(input)
  val propLastWinsAndOrthogonal = {
    val es = serializeTree(input).entries
    val changed = List(prop("renamed", Text("old"))).concat(es).append(prop("renamed", extra))
    lookup(changed, "renamed") == extra and argsOf(changed) == argsOf(es)
  }
  val argRoundTrip = readArgs(input, serializeTree(input)) == input.args.foldl(List(), (out, a) => out.append(a.value))
  val enumNewtypeRoundTrip = readEnum(writeEnum(enumValue)) == enumValue
  val stringTokenRoundTrip = parseValueToken(writeString(string)) == Text(string.text)
  val allInvariants = and { writerMatchesBuilder, propLastWinsAndOrthogonal, argRoundTrip, enumNewtypeRoundTrip, stringTokenRoundTrip }

  // 比較モードでも「欠点が期待どおり現れる」ことを確認するのでシナリオはすべて pass。
  run basicStructTest = init.then(setCase(sample(Text("a"), Text("b"), Text("p"), true, false),
    { tag: "idle", payload: "" }, API, Null)).expect(and {
      writerMatchesBuilder,
      argRoundTrip == STRICT_ARG_ORDER,
      required(lookup(serializeTree(input).entries, "renamed")) == Text("required"),
      lookup(serializeTree(input).entries, "optional") == Text("p"),
      childArg(serializeTree(input).children, "inner") == Text("child"),
      childArg(serializeTree(input).children, "unwrapped") == Text("scalar"),
      readChildren(serializeTree(input).children, false) == List(leaf("item", "one"), leaf("item", "two")),
      readStruct(input, serializeTree(input)) == {
        args: if (STRICT_ARG_ORDER) List(Text("a"), Text("b")) else List(Text("b"), Text("a")),
        requiredProp: Text("required"), optionalProp: Text("p"),
        child: Text("child"), unwrapped: Text("scalar"), items: List(Text("one"), Text("two")),
        flat: Null, deep: Null, flatChild: Null, deepChild: Null,
      },
    })
  run flattenTest = init.then(setCase(sample(Text("a"), Text("a"), Null, true, true),
    { tag: "idle", payload: "" }, API, Null)).expect(and {
      writerMatchesBuilder, argRoundTrip,
      lookup(serializeTree(input).entries, "flat") == Text("outer"),
      lookup(serializeTree(input).entries, "deep") == Text("inner"),
      childArg(serializeTree(input).children, "flat-child") == Text("outer"),
      childArg(serializeTree(input).children, "deep-child") == Text("inner"),
      serializeTree(input).children.length() == 6,
      readStruct(input, serializeTree(input)) == {
        args: List(Text("a"), Text("a")), requiredProp: Text("required"), optionalProp: Null,
        child: Text("child"), unwrapped: Text("scalar"), items: List(Text("one"), Text("two")),
        flat: Text("outer"), deep: Text("inner"), flatChild: Text("outer"), deepChild: Text("inner"),
      },
    })
  run duplicatePropertyTest = init.expect(and {
    lookup(List(prop("x", Text("first")), arg(Text("a")), prop("x", Text("last"))), "x") == Text("last"),
    propLastWinsAndOrthogonal,
  })
  run optionalArgumentTest = init.then(setCase(sample(Null, Text("b"), Null, false, false),
    { tag: "idle", payload: "" }, API, Null)).expect(and {
      argsOf(serializeTree(input).entries) == (if (NULL_FOR_NONE_ARG) List(Null, Text("b")) else List(Text("b"))),
      argRoundTrip == (NULL_FOR_NONE_ARG and STRICT_ARG_ORDER),
      serializeTree(input).entries.select(e => e.key == "optional").length() == 0,
      childArg(serializeTree(input).children, "inner") == Null,
    })
  run defaultsAndRequiredTest = init.expect(and {
    required(at(List(), 0)) == Invalid,
    required(lookup(List(), "renamed")) == Invalid,
    defaulted(List(), 0) == Text(""),
    defaulted(List(Null), 0) == Null,
  })
  run explicitIndexTest = init.expect(
    readArgs({ ...input, args: List({ value: Text("a"), index: 0 }, { value: Text("b"), index: 1 }) },
      serializeTree({ ...input, args: List({ value: Text("a"), index: 0 }, { value: Text("b"), index: 1 }) })) == List(Text("a"), Text("b")))
  // arguments と unwrap_args は位置引数全体を収集する。単一引数との混在を勝手に消費しない。
  run multiArgumentsTest = init.expect({
    val values = List(Text("a"), Text("b"))
    val x = { ...input, args: List(), rest: values, children: List(writeUnwrapArgs("unwrapped-list", values)) }
    and {
      serializeTree(x) == serializeDirect(x),
      argsOf(serializeTree(x).entries) == values,
      readUnwrapArgs(serializeTree(x).children, "unwrapped-list") == values,
      readUnwrapArgs(List(writeUnwrapArgs("unwrapped-list", List())), "unwrapped-list") == List(),
      readUnwrapArgs(List(), "unwrapped-list") == List(),
    }
  })
  run flattenArgumentsTest = init.expect({
    val x = { ...input, flattens: List({ args: List(Text("tail")),
      props: List({ key: "flat", value: Text("v"), optional: false }), children: List(leaf("flat-child", "c")) }) }
    and {
      serializeTree(x) == serializeDirect(x),
      argsOf(serializeTree(x).entries) == List(Text("a"), Text("a"), Text("tail")),
      lookup(serializeTree(x).entries, "flat") == Text("v"),
      childArg(serializeTree(x).children, "flat-child") == Text("c"),
    }
  })
  run dataEnumTest = init.expect(and {
    readEnum(writeEnum({ tag: "idle", payload: "" })) == { tag: "idle", payload: "" },
    readEnum(writeEnum({ tag: "named", payload: "x" })) == { tag: "named", payload: "x" },
    (readEnum(writeEnum({ tag: "wrapped", payload: "y" })) == { tag: "wrapped", payload: "y" }) == UNNAMED_NEWTYPE_ENUM,
    readChildren(List(writeEnum({ tag: "idle", payload: "" }), writeEnum({ tag: "named", payload: "x" }), writeEnum({ tag: "wrapped", payload: "y" })), true)
      .foldl(List(), (out, n) => out.append(readEnum(n))) == List({ tag: "idle", payload: "" }, { tag: "named", payload: "x" },
        if (UNNAMED_NEWTYPE_ENUM) { tag: "wrapped", payload: "y" } else { tag: "UnexpectedNode", payload: "" }),
  })
  run stringTokensTest = init.expect(and {
    isBare({ text: "-.x", chars: List("-", ".", "x") }),
    CANDIDATES.filter(l => Set(".5", "-1", "true", "null", "inf").contains(l.text))
      .forall(l => writeString(l).quoted == QUOTE_AMBIGUOUS_IDENT),
    CANDIDATES.forall(l => not(QUOTE_AMBIGUOUS_IDENT) or parseValueToken(writeString(l)) == Text(l.text)),
    parseValueToken({ lexeme: { text: "true", chars: List("t", "r", "u", "e") }, quoted: false }) == Invalid,
    parseValueToken({ lexeme: { text: "#true", chars: List("#", "t", "r", "u", "e") }, quoted: false }) == BooleanValue(true),
  })
}
module naive {
  import deriveModel(NULL_FOR_NONE_ARG=false, UNNAMED_NEWTYPE_ENUM=false, STRICT_ARG_ORDER=true, QUOTE_AMBIGUOUS_IDENT=false).*
}
module omitNoneArg {
  import deriveModel(NULL_FOR_NONE_ARG=false, UNNAMED_NEWTYPE_ENUM=true, STRICT_ARG_ORDER=true, QUOTE_AMBIGUOUS_IDENT=true).*
}
module namedNewtype {
  import deriveModel(NULL_FOR_NONE_ARG=true, UNNAMED_NEWTYPE_ENUM=false, STRICT_ARG_ORDER=true, QUOTE_AMBIGUOUS_IDENT=true).*
}
module looseIdent {
  import deriveModel(NULL_FOR_NONE_ARG=true, UNNAMED_NEWTYPE_ENUM=true, STRICT_ARG_ORDER=true, QUOTE_AMBIGUOUS_IDENT=false).*
}
module misorderedArgIndex {
  import deriveModel(NULL_FOR_NONE_ARG=true, UNNAMED_NEWTYPE_ENUM=true, STRICT_ARG_ORDER=false, QUOTE_AMBIGUOUS_IDENT=true).*
}
module clubKdlCurrent {
  import deriveModel(NULL_FOR_NONE_ARG=true, UNNAMED_NEWTYPE_ENUM=true, STRICT_ARG_ORDER=true, QUOTE_AMBIGUOUS_IDENT=true).*
}
