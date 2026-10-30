# club-kdl Guides

English | **[日本語](./README.ja.md)**

In-depth guides for using club-kdl. While the top-level `README.md` is an
"overview", these cover "procedures and decision criteria for specific tasks".

| Guide | Contents |
|-------|----------|
| [Rule-based Automatic Aliases](./01-compact-names.md) | Opt-in compact KDL files and messages, reusable name tables, and readable configuration |
| [Custom Types Guide](./custom-types.md) | Implement `FromKdlValue` / `ToKdlValue` to map your own types (chrono types, newtypes, external crate types) to KDL values |
| [KDL Design Best Practices](./best-practices.md) | Choosing between argument / property / children, and idiomatic KDL schema design with anti-patterns |
| [Troubleshooting](./troubleshooting.md) | Causes and fixes for common errors (lifetimes, child node resolution, type mismatches, etc.) |
