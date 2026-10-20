# KDL spec compliance test cases

`input/` and `expected_kdl/` are the official KDL 2.0 test cases, copied from
the [kdl-rs](https://github.com/kdl-org/kdl-rs) 6.7.1 source distribution
(`tests/test_cases/`, Apache-2.0, Copyright 2020 Kat Marchán), which in turn
mirrors [kdl-org/kdl](https://github.com/kdl-org/kdl).

`tests/spec_compliance.rs` checks club-kdl's parser against them: an input
with an `expected_kdl` counterpart must parse to the same tree; an input
without one must fail to parse.
