// mem_1ChPHcUK47Kmzw3sraumN9
// 有限の設計モデル。Rust parser / filesystem / SHA-256 の証明ではない。
module featureModel {
  const REPARSE_SPANS_ON_EDIT: bool
  const NO_ALIAS_SHADOWING: bool
  const EXPLICIT_SHORT_REF: bool
  const FILTER_BEFORE_RENAME: bool

  // edit: immutable source fragments + replaceable value fragments.
  // Each element is one abstract byte. Active paths are assigned after slashdash
  // exclusion, as parse_document_with_spans does; lexical parsing is a premise.
  type Fragment = { path: List[int], key: str, active: bool, bytes: List[str] }
  type Request = { path: List[int], key: str, index: int }
  pure def inert(bs: List[str]): Fragment = { path: List(), key: "", active: false, bytes: bs }
  pure def value(p: List[int], k: str, bs: List[str]): Fragment = { path: p, key: k, active: true, bytes: bs }
  pure val ORIGINAL = List(
    inert(List("/", "/", "c", "\n", "/", "-", "x", " ", "0", ";", "n", " ")),
    value(List(0), "", List("1")),
    inert(List(" ", "/", "-", "9", " ", "p", "=")),
    value(List(0), "p", List("2")), inert(List(" ", "p", "=")),
    value(List(0), "p", List("3")),
    inert(List(" ", "/", "-", "p", "=", "8", " ", "{", "c", " ")),
    value(List(0, 0), "", List("4")),
    inert(List(";", "}", "\n", "o", " ")),
    value(List(1), "", List("5")), inert(List(";", "\n")))
  pure def sourceOf(fs: List[Fragment]): List[str] = fs.foldl(List(), (out, f) => out.concat(f.bytes))
  pure def locate(fs: List[Fragment], r: Request): int = {
    val matches = fs.indices().fold(Set(), (out, i) =>
      if (fs[i].active and fs[i].path == r.path and fs[i].key == r.key) out.union(Set(i)) else out)
    // indices are sorted explicitly: Set fold order must not decide rightmost.
    val ordered = range(0, fs.length()).select(i => matches.contains(i))
    if (r.key == "") { if (r.index >= 0 and r.index < ordered.length()) ordered[r.index] else -1 }
    else if (ordered.length() > 0) ordered[ordered.length() - 1] else -1
  }
  pure def offset(fs: List[Fragment], i: int): int = fs.slice(0, i).foldl(0, (n, f) => n + f.bytes.length())
  type Edit = { source: List[str], oracle: List[Fragment], before: List[str], error: str }
  pure val EDIT_INIT = { source: sourceOf(ORIGINAL), oracle: ORIGINAL, before: sourceOf(ORIGINAL), error: "" }
  pure def edit(e: Edit, r: Request, replacement: List[str]): Edit = {
    val i = locate(e.oracle, r)
    if (i < 0) {
      val missing = if (not(Set(List(0), List(0, 0), List(1)).contains(r.path))) "MissingNode"
        else if (r.key == "") "MissingArgument" else "MissingProperty"
      { ...e, before: e.source, error: missing }
    } else {
      // On the valid current path, reparsing yields the current fragment spans.
      // The cached variant intentionally reads INITIAL spans, not current ones.
      val spans = if (REPARSE_SPANS_ON_EDIT) e.oracle else ORIGINAL
      val start = offset(spans, i)
      val end = start + spans[i].bytes.length()
      val updated = if (end <= e.source.length())
        e.source.slice(0, start).concat(replacement).concat(e.source.slice(end, e.source.length()))
        else e.source // cached out-of-range is a failed implementation, not a Quint runtime error
      { source: updated, oracle: e.oracle.replaceAt(i, { ...e.oracle[i], bytes: replacement }),
        before: e.source, error: "" }
    }
  }
  pure val ARG0 = { path: List(0), key: "", index: 0 }
  pure val PROP = { path: List(0), key: "p", index: 0 }
  pure val CHILD = { path: List(0, 0), key: "", index: 0 }
  pure val OTHER = { path: List(1), key: "", index: 0 }
  pure val MISSING_NODE = { path: List(2), key: "", index: 0 }
  pure val MISSING_ARG = { path: List(0), key: "", index: 1 }
  pure val MISSING_PROP = { path: List(0), key: "absent", index: 0 }
  pure val EMPTY_PATH = { path: List(), key: "", index: 0 }

  // compact: finite sorted vocabulary and shortest ASCII candidate aliases.
  type Binding = { original: str, alias: str }
  type Named = { node: str, prop: str, payload: str }
  type Decoded = { error: str, nodes: List[Named] }
  pure def table(names: List[str]): List[Binding] = {
    names.foldl(List(), (out, name) => {
      val available = List("a", "b", "c", "d", "e").select(a =>
        (not(NO_ALIAS_SHADOWING) or not(names.indices().exists(i => names[i] == a)))
          and not(out.indices().exists(i => out[i].alias == a and out[i].original != "a")))
      // The naive generator neglects the reserved original 'a'.
      out.append({ original: name, alias: if (name == "a") name else available[0] })
    })
  }
  pure def lookup(t: List[Binding], n: str, decode: bool): str = t.foldl("?", (out, b) =>
    if ((if (decode) b.alias else b.original) == n) (if (decode) b.original else b.alias) else out)
  pure def remap(t: List[Binding], ns: List[Named], decode: bool): List[Named] = ns.foldl(List(), (out, n) =>
    out.append({ ...n, node: lookup(t, n.node, decode), prop: lookup(t, n.prop, decode) }))
  pure def decode(t: List[Binding], messageId: List[Binding], ns: List[Named]): Decoded =
    if (t != messageId) { error: "TableMismatch", nodes: List() }
    else {
      val decoded = remap(t, ns, true)
      if (decoded.indices().exists(i => decoded[i].node == "?" or decoded[i].prop == "?"))
        { error: "UnknownAlias", nodes: List() }
      else { error: "", nodes: decoded }
    }
  pure def validTable(t: List[Binding]): bool = t.indices().forall(i => t.indices().forall(j =>
    (i == j or (t[i].original != t[j].original and t[i].alias != t[j].alias)) and
    (not(NO_ALIAS_SHADOWING) or t[i].alias == t[i].original or t[i].alias != t[j].original)))
  type Compact = { actual: Decoded, expected: Decoded, loaded: bool, expectedLoad: bool }
  pure def compactCase(which: int): Compact = {
    val names = if (which == 0) List("long_name", "other_name") else List("a", "long_name", "other_name")
    val t = table(names)
    val ns = List({ node: names[0], prop: "long_name", payload: "a" },
      { node: "other_name", prop: names[0], payload: "long_name" })
    val encoded = remap(t, ns, false)
    val swapped = List({ original: "a", alias: "b" }, { original: "b", alias: "a" })
    val message = if (which == 3) encoded.replaceAt(1, { ...encoded[1], prop: "?" })
      else if (which == 4) encoded.replaceAt(0, { ...encoded[0], node: "?" }) else encoded
    val id = if (which == 2) List({ original: "different", alias: "z" }) else t
    { actual: decode(t, id, message),
      expected: if (which == 2) { error: "TableMismatch", nodes: List() }
        else if (which == 3 or which == 4) { error: "UnknownAlias", nodes: List() }
        else { error: "", nodes: ns },
      loaded: validTable(if (which == 5) swapped else t), expectedLoad: which != 5 }
  }

  // compose: relative paths are segment lists; final segment has a finite
  // spelling table for extension detection (not a reimplementation of Path).
  type CNode = { name: str, tag: str, args: List[str], argTyped: bool }
  pure def data(n: str): CNode = { name: n, tag: "", args: List(), argTyped: false }
  pure def short(p: str): CNode = { name: ">", tag: "", args: List(p), argTyped: false }
  type Directive = { kind: str, request: str }
  pure def directive(n: CNode): Directive =
    if (n.name == ">" and n.tag == "") {
      if (n.args.length() != 1 or n.argTyped) { kind: "InvalidDirective", request: "" }
      else { kind: "short", request: n.args[0] }
    } else if (n.name == "file" and n.tag == "<") {
      if (n.args.length() == 0) { kind: "InvalidDirective", request: "" }
      else { kind: "literal", request: n.args[0] }
    } else if (not(EXPLICIT_SHORT_REF) and n.tag == "" and n.args.length() == 0)
      { kind: "short", request: n.name }
    else { kind: "data", request: "" }
  pure def resolvePath(from: List[str], d: Directive): List[str] = {
    val filename = if (d.kind == "short" and not(Set("a.data", "types.kdl").contains(d.request)))
      if (d.request == "types") "types.kdl"
      else if (d.request == "B") "B.kdl"
      else if (d.request == "C") "C.kdl"
      else if (d.request == "D") "D.kdl"
      else if (d.request == "loop") "loop.kdl"
      else if (d.request == "A") "A.kdl"
      else if (d.request == "a") "a.kdl"
      else "enabled.kdl"
    else d.request
    from.slice(0, from.length() - 1).append(filename)
  }
  type File = { path: List[str], nodes: List[CNode] }
  type Work = { kind: str, file: List[str], node: CNode }
  type Walk = { pending: List[Work], stack: List[List[str]], output: List[CNode], error: str }
  pure def enter(p: List[str]): Work = { kind: "enter", file: p, node: data("") }
  pure def walkStep(fs: List[File], w: Walk): Walk =
    if (w.pending.length() == 0 or w.error != "") w else {
      val work = w.pending[0]
      val rest = w.pending.tail()
      if (work.kind == "leave") { ...w, pending: rest, stack: w.stack.slice(0, w.stack.length() - 1) }
      else if (work.kind == "node") {
        val d = directive(work.node)
        if (d.kind == "data") { ...w, pending: rest, output: w.output.append(work.node) }
        else if (d.kind == "InvalidDirective") { ...w, error: d.kind }
        else { ...w, pending: List(enter(resolvePath(work.file, d))).concat(rest) }
      } else if (w.stack.indices().exists(i => w.stack[i] == work.file)) { ...w, error: "Cycle" }
      else {
        val found = fs.select(f => f.path == work.file)
        if (found.length() == 0) { ...w, error: "Io" }
        else {
          val items = found[0].nodes.foldl(List(), (out, n) => out.append({ kind: "node", file: work.file, node: n }))
          { ...w, stack: w.stack.append(work.file),
            pending: items.append({ ...work, kind: "leave" }).concat(rest) }
        }
      }
    }
  type Composed = { error: str, nodes: List[CNode] }
  pure def compose(fs: List[File], root: List[str]): Composed = {
    val initial = { pending: List(enter(root)), stack: List(), output: List(), error: "" }
    val done = range(0, 40).foldl(initial, (w, _) => walkStep(fs, w))
    if (done.error != "") { error: done.error, nodes: List() }
    else if (done.pending.length() > 0 or done.stack.length() > 0) { error: "FuelExhausted", nodes: List() }
    else { error: "", nodes: done.output }
  }
  pure val LEAF = { name: "type", tag: "", args: List("User"), argTyped: false }
  pure val LITERAL = { name: "file", tag: "<", args: List("literal"), argTyped: false }
  pure def files(which: int): List[File] = {
    val base = List("pkg", "sub")
    val rootNodes = if (which == 0) List(LEAF)
      else if (which == 1) List(data("enabled"), data("a"))
      else if (which == 2) List(short("types"), short("a.data"), LITERAL)
      else if (which == 3) List(short("B"), short("C"))
      else if (which == 4) List(short("loop"))
      else if (which == 5) List({ ...short("types"), tag: "custom" })
      else if (which == 6) List({ ...short("types"), argTyped: true })
      else List({ ...short("types"), args: List("types", "extra") })
    List({ path: base.append("A.kdl"), nodes: rootNodes },
      { path: base.append("types.kdl"), nodes: List(LEAF) },
      { path: base.append("a.data"), nodes: List({ ...LEAF, args: List("Data") }) },
      { path: base.append("literal"), nodes: List({ ...LEAF, args: List("Literal") }) },
      { path: base.append("B.kdl"), nodes: List(short("D")) },
      { path: base.append("C.kdl"), nodes: List(short("D")) },
      { path: base.append("D.kdl"), nodes: List(LEAF) },
      { path: base.append("loop.kdl"), nodes: List(short("A")) },
      // A wrong cwd-based resolver would read this decoy instead.
      { path: List("types.kdl"), nodes: List({ ...LEAF, args: List("WrongDirectory") }) })
  }
  pure def composeExpected(which: int): Composed =
    if (which == 1) { error: "", nodes: List(data("enabled"), data("a")) }
    else if (which == 2) { error: "", nodes: List(LEAF, { ...LEAF, args: List("Data") }, { ...LEAF, args: List("Literal") }) }
    else if (which == 3) { error: "", nodes: List(LEAF, LEAF) }
    else if (which == 4) { error: "Cycle", nodes: List() }
    else if (which == 5) { error: "", nodes: List({ ...short("types"), tag: "custom" }) }
    else if (which >= 6) { error: "InvalidDirective", nodes: List() }
    else { error: "", nodes: List(LEAF) }

  // Filters use the FIRST positional string, never the KDL node name.
  type MetaNode = { node: str, first: str, hasString: bool, payload: str }
  type Options = { onlyOn: bool, only: Set[str], except: Set[str], rename: bool, prefix: bool }
  pure def renamed(n: MetaNode, o: Options): MetaNode =
    if (not(n.hasString)) n else {
      val name = if (o.rename and n.first == "User") "Acct" else n.first
      { ...n, first: if (o.prefix) if (name == "Acct") "shared.Acct" else if (name == "User") "shared.User" else "shared.Order" else name }
    }
  pure def accepted(n: MetaNode, o: Options): bool = not(n.hasString) or
    ((not(o.onlyOn) or o.only.contains(n.first)) and not(o.except.contains(n.first)))
  pure def transform(ns: List[MetaNode], o: Options): List[MetaNode] = ns.foldl(List(), (out, n) => {
    val changed = renamed(n, o)
    if (accepted(if (FILTER_BEFORE_RENAME) n else changed, o)) out.append(changed) else out
  })
  // Independent reference pipeline: selection once, then two separate passes.
  pure def transformExpected(ns: List[MetaNode], o: Options): List[MetaNode] = ns
    .select(n => not(n.hasString) or ((not(o.onlyOn) or o.only.contains(n.first)) and not(o.except.contains(n.first))))
    .foldl(List(), (out, n) => out.append(if (n.hasString and o.rename and n.first == "User") { ...n, first: "Acct" } else n))
    .foldl(List(), (out, n) => out.append(if (n.hasString and o.prefix) { ...n, first: if (n.first == "User") "shared.User" else if (n.first == "Order") "shared.Order" else "shared.Acct" } else n))
  pure val META = { node: "kdl-version", first: "", hasString: false, payload: "2" }
  pure val TYPES = List({ node: "type", first: "User", hasString: true, payload: "body" },
    { node: "type", first: "Order", hasString: true, payload: "body2" }, META)
  pure val OPTIONS = { onlyOn: true, only: Set("User"), except: Set(), rename: true, prefix: true }

  var edited: Edit
  var packed: Compact
  var composed: Composed
  var expectedCompose: Composed
  var transformed: List[MetaNode]
  var expectedTransform: List[MetaNode]
  action init = all {
    edited' = EDIT_INIT, packed' = compactCase(0),
    composed' = composeExpected(0), expectedCompose' = composeExpected(0),
    transformed' = TYPES, expectedTransform' = TYPES,
  }
  action editAt(r: Request, bs: List[str]): bool = all {
    edited' = edit(edited, r, bs), packed' = packed, composed' = composed,
    expectedCompose' = expectedCompose, transformed' = transformed, expectedTransform' = expectedTransform,
  }
  action packCase(i: int): bool = all {
    edited' = edited, packed' = compactCase(i), composed' = composed,
    expectedCompose' = expectedCompose, transformed' = transformed, expectedTransform' = expectedTransform,
  }
  action composeCase(i: int): bool = all {
    edited' = edited, packed' = packed, composed' = compose(files(i), List("pkg", "sub", "A.kdl")),
    expectedCompose' = composeExpected(i), transformed' = transformed, expectedTransform' = expectedTransform,
  }
  action transformCase(o: Options): bool = all {
    edited' = edited, packed' = packed, composed' = composed, expectedCompose' = expectedCompose,
    transformed' = transform(TYPES, o), expectedTransform' = transformExpected(TYPES, o),
  }
  action step = any {
    { nondet r = Set(ARG0, PROP, CHILD, OTHER, MISSING_NODE, MISSING_ARG, MISSING_PROP, EMPTY_PATH).oneOf()
      nondet bs = Set(List("6"), List("1", "2", "3"), List("7", "8")).oneOf()
      editAt(r, bs) },
    { nondet i = 0.to(5).oneOf(); packCase(i) },
    { nondet i = 0.to(7).oneOf(); composeCase(i) },
    { nondet onlyOn = Set(true, false).oneOf()
      nondet only = Set(Set("User"), Set("Order"), Set()).oneOf()
      nondet except = Set(Set("User"), Set("Order"), Set()).oneOf()
      nondet rename = Set(true, false).oneOf()
      nondet prefix = Set(true, false).oneOf()
      transformCase({ onlyOn: onlyOn, only: only, except: except, rename: rename, prefix: prefix }) },
  }
  val editPreservesUntouchedAndSlashdash = edited.source == sourceOf(edited.oracle)
    and (edited.error == "" or edited.source == edited.before)
  val compactBijectiveAndSafe = packed.actual == packed.expected and packed.loaded == packed.expectedLoad
  val composeDataAndCycleSafe = composed == expectedCompose
  val composeTransformOrderCorrect = transformed == expectedTransform
  val allFeatureInvariants = and { editPreservesUntouchedAndSlashdash, compactBijectiveAndSafe, composeDataAndCycleSafe, composeTransformOrderCorrect }

  run consecutiveEditTest = init.then(editAt(ARG0, List("1", "2", "3"))).expect(editPreservesUntouchedAndSlashdash)
    .then(editAt(PROP, List("7", "8"))).expect(editPreservesUntouchedAndSlashdash == REPARSE_SPANS_ON_EDIT)
    .then(editAt(PROP, List("6"))).expect(editPreservesUntouchedAndSlashdash == REPARSE_SPANS_ON_EDIT)
  run rightmostPropertyTest = init.then(editAt(PROP, List("7", "8"))).expect(and {
    editPreservesUntouchedAndSlashdash, edited.oracle[3].bytes == List("2"), edited.oracle[5].bytes == List("7", "8"),
    edited.source == sourceOf(ORIGINAL.replaceAt(5, { ...ORIGINAL[5], bytes: List("7", "8") })),
  })
  run slashdashAndNestedTest = init.then(editAt(CHILD, List("6"))).then(editAt(OTHER, List("7"))).expect(and {
    editPreservesUntouchedAndSlashdash,
    edited.source == sourceOf(ORIGINAL.replaceAt(7, { ...ORIGINAL[7], bytes: List("6") }).replaceAt(9, { ...ORIGINAL[9], bytes: List("7") })),
    locate(ORIGINAL, ARG0) == 1, locate(ORIGINAL, MISSING_ARG) == -1,
  })
  run failedEditsAtomicTest = init.then(editAt(ARG0, List("1", "2", "3")))
    .then(editAt(MISSING_NODE, List("9"))).expect(edited.error == "MissingNode" and edited.source == edited.before)
    .then(editAt(MISSING_ARG, List("9"))).expect(edited.error == "MissingArgument" and edited.source == edited.before)
    .then(editAt(MISSING_PROP, List("9"))).expect(edited.error == "MissingProperty" and edited.source == edited.before)
    .then(editAt(EMPTY_PATH, List("9"))).expect(edited.error == "MissingNode" and editPreservesUntouchedAndSlashdash)
  run shadowingAndRoundTripTest = init.then(packCase(1)).expect(and {
    (packed.actual == packed.expected) == NO_ALIAS_SHADOWING,
    (lookup(table(List("a", "long_name", "other_name")), "long_name", false) != "a") == NO_ALIAS_SHADOWING,
    packed.loaded == NO_ALIAS_SHADOWING,
  })
  run tableMismatchTest = init.then(packCase(2)).expect(and {
    packed.actual.error == "TableMismatch", packed.actual.nodes == List(),
    // ID validation precedes name lookup, even with unknown aliases.
    decode(table(List("long_name")), List(), List({ node: "?", prop: "?", payload: "x" })).error == "TableMismatch",
  })
  run unknownAliasTest = init.then(packCase(3)).expect(packed.actual.error == "UnknownAlias")
    .then(packCase(4)).expect(packed.actual.error == "UnknownAlias")
  run tableValidationTest = init.then(packCase(5)).expect(and {
    packed.loaded == not(NO_ALIAS_SHADOWING),
    not(validTable(List({ original: "x", alias: "z" }, { original: "y", alias: "z" }))),
    not(validTable(List({ original: "x", alias: "z" }, { original: "x", alias: "y" }))),
    validTable(List({ original: "a", alias: "a" }, { original: "long_name", alias: "b" })),
  })
  run emptyDataNodesTest = init.then(composeCase(1)).expect(and {
    composeDataAndCycleSafe == EXPLICIT_SHORT_REF,
    directive(data("enabled")).kind == (if (EXPLICIT_SHORT_REF) "data" else "short"),
    directive(data("a")).kind == (if (EXPLICIT_SHORT_REF) "data" else "short"),
    composed.error == (if (EXPLICIT_SHORT_REF) "" else "Io"),
  })
  run pathsAndLiteralTest = init.then(composeCase(2)).expect(and {
    composeDataAndCycleSafe,
    resolvePath(List("pkg", "sub", "A.kdl"), directive(short("types"))) == List("pkg", "sub", "types.kdl"),
    resolvePath(List("pkg", "sub", "A.kdl"), directive(short("types.kdl"))) == List("pkg", "sub", "types.kdl"),
    resolvePath(List("pkg", "sub", "A.kdl"), directive(short("a.data"))) == List("pkg", "sub", "a.data"),
    resolvePath(List("pkg", "sub", "A.kdl"), directive(LITERAL)) == List("pkg", "sub", "literal"),
  })
  run diamondTest = init.then(composeCase(3)).expect(composeDataAndCycleSafe and composed.nodes == List(LEAF, LEAF))
  run cycleTest = init.then(composeCase(4)).expect(composeDataAndCycleSafe and composed.error == "Cycle")
  run typedAndInvalidReferenceTest = init.then(composeCase(5)).expect(composeDataAndCycleSafe)
    .then(composeCase(6)).expect(composeDataAndCycleSafe and composed.error == "InvalidDirective")
    .then(composeCase(7)).expect(composeDataAndCycleSafe and composed.error == "InvalidDirective")
  run onlyRenamePrefixTest = init.then(transformCase(OPTIONS)).expect(and {
    composeTransformOrderCorrect == FILTER_BEFORE_RENAME,
    transformed == (if (FILTER_BEFORE_RENAME) List({ ...TYPES[0], first: "shared.Acct" }, META) else List(META)),
  })
  run exceptRenamePrefixTest = init.then(transformCase({ ...OPTIONS, onlyOn: false, except: Set("User") })).expect(and {
    composeTransformOrderCorrect == FILTER_BEFORE_RENAME,
    transformed == (if (FILTER_BEFORE_RENAME) List({ ...TYPES[1], first: "shared.Order" }, META)
      else List({ ...TYPES[0], first: "shared.Acct" }, { ...TYPES[1], first: "shared.Order" }, META)),
  })
  run metaPassThroughTest = init.then(transformCase({ ...OPTIONS, only: Set(), except: Set("User", "Order") }))
    .expect(transformed == List(META) and composeTransformOrderCorrect)
}
module naiveFeatures {
  import featureModel(REPARSE_SPANS_ON_EDIT=false, NO_ALIAS_SHADOWING=false, EXPLICIT_SHORT_REF=false, FILTER_BEFORE_RENAME=false).*
}
module staleEditSpans {
  import featureModel(REPARSE_SPANS_ON_EDIT=false, NO_ALIAS_SHADOWING=true, EXPLICIT_SHORT_REF=true, FILTER_BEFORE_RENAME=true).*
}
module shadowedCompact {
  import featureModel(REPARSE_SPANS_ON_EDIT=true, NO_ALIAS_SHADOWING=false, EXPLICIT_SHORT_REF=true, FILTER_BEFORE_RENAME=true).*
}
module implicitComposeRef {
  import featureModel(REPARSE_SPANS_ON_EDIT=true, NO_ALIAS_SHADOWING=true, EXPLICIT_SHORT_REF=false, FILTER_BEFORE_RENAME=true).*
}
module renameBeforeFilter {
  import featureModel(REPARSE_SPANS_ON_EDIT=true, NO_ALIAS_SHADOWING=true, EXPLICIT_SHORT_REF=true, FILTER_BEFORE_RENAME=false).*
}
module clubKdlFeaturesCurrent {
  import featureModel(REPARSE_SPANS_ON_EDIT=true, NO_ALIAS_SHADOWING=true, EXPLICIT_SHORT_REF=true, FILTER_BEFORE_RENAME=true).*
}
