//! Exhaustive KDL ↔ Struct mapping tests
//!
//! Every `#[kdl(...)]` attribute combination is tested here.
//! Each section tests: Deserialize, Serialize (where applicable), and Roundtrip.

use std::collections::HashMap;
use std::path::PathBuf;
use unison_kdl::{FromKdlValue, KdlDeserialize, KdlNodeExt, KdlSerialize, KdlValue};

// ============================================================================
// 1. Argument patterns
// ============================================================================

mod argument {
    use super::*;

    // --- 1a. Single argument (auto-index) ---

    #[derive(Debug, PartialEq, KdlDeserialize, KdlSerialize)]
    #[kdl(name = "item")]
    struct SingleArg {
        #[kdl(argument)]
        name: String,
    }

    #[test]
    fn de_single_arg() {
        let v: SingleArg = unison_kdl::from_str(r#"item "hello""#).unwrap();
        assert_eq!(v.name, "hello");
    }

    #[test]
    fn roundtrip_single_arg() {
        let original = SingleArg {
            name: "test".into(),
        };
        let node = original.to_kdl_node().unwrap();
        let restored = SingleArg::from_kdl_node(&node).unwrap();
        assert_eq!(original, restored);
    }

    // --- 1b. Multiple arguments (auto-index increments) ---

    #[derive(Debug, PartialEq, KdlDeserialize, KdlSerialize)]
    #[kdl(name = "pair")]
    struct MultiArg {
        #[kdl(argument)]
        first: String,
        #[kdl(argument)]
        second: i64,
    }

    #[test]
    fn de_multi_arg_auto_index() {
        let v: MultiArg = unison_kdl::from_str(r#"pair "key" 42"#).unwrap();
        assert_eq!(v.first, "key");
        assert_eq!(v.second, 42);
    }

    #[test]
    fn roundtrip_multi_arg() {
        let original = MultiArg {
            first: "a".into(),
            second: 99,
        };
        let node = original.to_kdl_node().unwrap();
        let restored = MultiArg::from_kdl_node(&node).unwrap();
        assert_eq!(original, restored);
    }

    // --- 1c. Explicit index ---

    #[derive(Debug, PartialEq, KdlDeserialize)]
    #[kdl(name = "indexed")]
    struct ExplicitIndex {
        #[kdl(argument(index = 1))]
        second: String,
        #[kdl(argument(index = 0))]
        first: i64,
    }

    #[test]
    fn de_explicit_index() {
        let v: ExplicitIndex = unison_kdl::from_str(r#"indexed 10 "world""#).unwrap();
        assert_eq!(v.first, 10);
        assert_eq!(v.second, "world");
    }

    // --- 1d. Optional argument ---

    #[derive(Debug, PartialEq, KdlDeserialize)]
    #[kdl(name = "maybe")]
    struct OptionalArg {
        #[kdl(argument)]
        required: String,
        #[kdl(argument)]
        optional: Option<i64>,
    }

    #[test]
    fn de_optional_arg_present() {
        let v: OptionalArg = unison_kdl::from_str(r#"maybe "hi" 5"#).unwrap();
        assert_eq!(v.optional, Some(5));
    }

    #[test]
    fn de_optional_arg_absent() {
        let v: OptionalArg = unison_kdl::from_str(r#"maybe "hi""#).unwrap();
        assert_eq!(v.optional, None);
    }

    // --- 1e. Default argument ---

    #[derive(Debug, PartialEq, KdlDeserialize)]
    #[kdl(name = "def")]
    struct DefaultArg {
        #[kdl(argument, default)]
        value: i64,
    }

    #[test]
    fn de_default_arg_present() {
        let v: DefaultArg = unison_kdl::from_str(r#"def 42"#).unwrap();
        assert_eq!(v.value, 42);
    }

    #[test]
    fn de_default_arg_absent() {
        let v: DefaultArg = unison_kdl::from_str(r#"def"#).unwrap();
        assert_eq!(v.value, 0); // i64::default()
    }

    // --- 1f. Arguments (collect all into Vec) ---

    #[derive(Debug, PartialEq, KdlDeserialize, KdlSerialize)]
    #[kdl(name = "tags")]
    struct AllArgs {
        #[kdl(arguments)]
        items: Vec<String>,
    }

    #[test]
    fn de_arguments_vec() {
        let v: AllArgs = unison_kdl::from_str(r#"tags "a" "b" "c""#).unwrap();
        assert_eq!(v.items, vec!["a", "b", "c"]);
    }

    #[test]
    fn de_arguments_empty() {
        let v: AllArgs = unison_kdl::from_str(r#"tags"#).unwrap();
        assert!(v.items.is_empty());
    }

    #[test]
    fn roundtrip_arguments() {
        let original = AllArgs {
            items: vec!["x".into(), "y".into()],
        };
        let node = original.to_kdl_node().unwrap();
        let restored = AllArgs::from_kdl_node(&node).unwrap();
        assert_eq!(original, restored);
    }
}

// ============================================================================
// 2. Property patterns
// ============================================================================

mod property {
    use super::*;

    // --- 2a. Basic property ---

    #[derive(Debug, PartialEq, KdlDeserialize, KdlSerialize)]
    #[kdl(name = "cfg")]
    struct BasicProp {
        #[kdl(property)]
        host: String,
        #[kdl(property)]
        port: u16,
    }

    #[test]
    fn de_basic_property() {
        let v: BasicProp = unison_kdl::from_str(r#"cfg host="localhost" port=8080"#).unwrap();
        assert_eq!(v.host, "localhost");
        assert_eq!(v.port, 8080);
    }

    #[test]
    fn roundtrip_basic_property() {
        let original = BasicProp {
            host: "0.0.0.0".into(),
            port: 3000,
        };
        let node = original.to_kdl_node().unwrap();
        let restored = BasicProp::from_kdl_node(&node).unwrap();
        assert_eq!(original, restored);
    }

    // --- 2b. Renamed property ---

    #[derive(Debug, PartialEq, KdlDeserialize, KdlSerialize)]
    #[kdl(name = "db")]
    struct RenamedProp {
        #[kdl(property(rename = "connection-string"))]
        connection_string: String,
    }

    #[test]
    fn de_renamed_property() {
        let v: RenamedProp =
            unison_kdl::from_str(r#"db connection-string="postgres://localhost""#).unwrap();
        assert_eq!(v.connection_string, "postgres://localhost");
    }

    #[test]
    fn roundtrip_renamed_property() {
        let original = RenamedProp {
            connection_string: "sqlite://db.sqlite".into(),
        };
        let node = original.to_kdl_node().unwrap();
        assert_eq!(
            node.prop("connection-string").and_then(|v| v.as_string()),
            Some("sqlite://db.sqlite")
        );
        let restored = RenamedProp::from_kdl_node(&node).unwrap();
        assert_eq!(original, restored);
    }

    // --- 2c. Optional property ---

    #[derive(Debug, PartialEq, KdlDeserialize, KdlSerialize)]
    #[kdl(name = "server")]
    struct OptionalProp {
        #[kdl(property)]
        host: String,
        #[kdl(property)]
        tls: Option<bool>,
    }

    #[test]
    fn de_optional_property_present() {
        let v: OptionalProp =
            unison_kdl::from_str(r#"server host="localhost" tls=#true"#).unwrap();
        assert_eq!(v.tls, Some(true));
    }

    #[test]
    fn de_optional_property_absent() {
        let v: OptionalProp = unison_kdl::from_str(r#"server host="localhost""#).unwrap();
        assert_eq!(v.tls, None);
    }

    #[test]
    fn roundtrip_optional_property_some() {
        let original = OptionalProp {
            host: "h".into(),
            tls: Some(true),
        };
        let node = original.to_kdl_node().unwrap();
        let restored = OptionalProp::from_kdl_node(&node).unwrap();
        assert_eq!(original, restored);
    }

    #[test]
    fn roundtrip_optional_property_none() {
        let original = OptionalProp {
            host: "h".into(),
            tls: None,
        };
        let node = original.to_kdl_node().unwrap();
        let restored = OptionalProp::from_kdl_node(&node).unwrap();
        assert_eq!(original, restored);
    }

    // --- 2d. Default property ---

    #[derive(Debug, PartialEq, KdlDeserialize)]
    #[kdl(name = "app")]
    struct DefaultProp {
        #[kdl(property, default)]
        debug: bool,
        #[kdl(property, default)]
        workers: i64,
    }

    #[test]
    fn de_default_property_present() {
        let v: DefaultProp = unison_kdl::from_str(r#"app debug=#true workers=4"#).unwrap();
        assert_eq!(v.debug, true);
        assert_eq!(v.workers, 4);
    }

    #[test]
    fn de_default_property_absent() {
        let v: DefaultProp = unison_kdl::from_str(r#"app"#).unwrap();
        assert_eq!(v.debug, false);
        assert_eq!(v.workers, 0);
    }
}

// ============================================================================
// 3. Child patterns
// ============================================================================

mod child {
    use super::*;

    // --- Helper structs ---

    #[derive(Debug, PartialEq, Clone, KdlDeserialize, KdlSerialize)]
    #[kdl(name = "database")]
    struct Database {
        #[kdl(argument)]
        url: String,
        #[kdl(property)]
        pool_size: Option<i64>,
    }

    // --- 3a. Required child (auto-name from child's #[kdl(name)]) ---

    #[derive(Debug, PartialEq, KdlDeserialize)]
    #[kdl(name = "app")]
    struct RequiredChild {
        #[kdl(child)]
        database: Database,
    }

    #[test]
    fn de_required_child_auto_name() {
        let v: RequiredChild = unison_kdl::from_str(
            r#"app {
                database "postgres://localhost"
            }"#,
        )
        .unwrap();
        assert_eq!(v.database.url, "postgres://localhost");
    }

    #[test]
    fn de_required_child_missing_errors() {
        let result = unison_kdl::from_str::<RequiredChild>(r#"app"#);
        assert!(result.is_err());
    }

    // --- 3b. Optional child (auto-name) ---

    #[derive(Debug, PartialEq, KdlDeserialize, KdlSerialize)]
    #[kdl(name = "app")]
    struct OptionalChild {
        #[kdl(argument)]
        name: String,
        #[kdl(child)]
        database: Option<Database>,
    }

    #[test]
    fn de_optional_child_present() {
        let v: OptionalChild = unison_kdl::from_str(
            r#"app "myapp" {
                database "sqlite://db" pool_size=5
            }"#,
        )
        .unwrap();
        assert!(v.database.is_some());
        assert_eq!(v.database.as_ref().unwrap().pool_size, Some(5));
    }

    #[test]
    fn de_optional_child_absent() {
        let v: OptionalChild = unison_kdl::from_str(r#"app "myapp""#).unwrap();
        assert!(v.database.is_none());
    }

    #[test]
    fn roundtrip_optional_child_some() {
        let original = OptionalChild {
            name: "test".into(),
            database: Some(Database {
                url: "pg://".into(),
                pool_size: Some(10),
            }),
        };
        let node = original.to_kdl_node().unwrap();
        let restored = OptionalChild::from_kdl_node(&node).unwrap();
        assert_eq!(original, restored);
    }

    #[test]
    fn roundtrip_optional_child_none() {
        let original = OptionalChild {
            name: "test".into(),
            database: None,
        };
        let node = original.to_kdl_node().unwrap();
        let restored = OptionalChild::from_kdl_node(&node).unwrap();
        assert_eq!(original, restored);
    }

    // --- 3c. Default child ---

    #[derive(Debug, Default, PartialEq, KdlDeserialize)]
    #[kdl(name = "limits")]
    struct Limits {
        #[kdl(property, default)]
        max_connections: i64,
        #[kdl(property, default)]
        timeout: i64,
    }

    #[derive(Debug, PartialEq, KdlDeserialize)]
    #[kdl(name = "server")]
    struct DefaultChild {
        #[kdl(child, default)]
        limits: Limits,
    }

    #[test]
    fn de_default_child_present() {
        let v: DefaultChild = unison_kdl::from_str(
            r#"server {
                limits max_connections=100 timeout=30
            }"#,
        )
        .unwrap();
        assert_eq!(v.limits.max_connections, 100);
    }

    #[test]
    fn de_default_child_absent() {
        let v: DefaultChild = unison_kdl::from_str(r#"server"#).unwrap();
        assert_eq!(v.limits, Limits::default());
    }

    // --- 3d. Explicit child name (overrides auto-name) ---

    #[derive(Debug, PartialEq, KdlDeserialize)]
    #[kdl(name = "app")]
    struct ExplicitChildName {
        #[kdl(child(name = "db"))]
        database: Option<Database>,
    }

    #[test]
    fn de_explicit_child_name() {
        // "db" is used for searching, but Database has #[kdl(name = "database")]
        // which triggers a name_check → UnexpectedNode.
        // This is expected: explicit child_name overrides SEARCH name only.
        // To use a different KDL name, the child struct itself needs matching #[kdl(name)].
        let result = unison_kdl::from_str::<ExplicitChildName>(
            r#"app {
                db "postgres://localhost"
            }"#,
        );
        // Fails because Database expects node name "database", not "db"
        assert!(result.is_err());
    }

    // For explicit name to work, child struct's #[kdl(name)] must match
    #[derive(Debug, PartialEq, KdlDeserialize)]
    #[kdl(name = "db")]
    struct DbAlias {
        #[kdl(argument)]
        url: String,
    }

    #[derive(Debug, PartialEq, KdlDeserialize)]
    #[kdl(name = "app")]
    struct ExplicitChildNameMatching {
        #[kdl(child(name = "db"))]
        database: Option<DbAlias>,
    }

    #[test]
    fn de_explicit_child_name_matching() {
        let v: ExplicitChildNameMatching = unison_kdl::from_str(
            r#"app {
                db "postgres://localhost"
            }"#,
        )
        .unwrap();
        assert!(v.database.is_some());
        assert_eq!(v.database.unwrap().url, "postgres://localhost");
    }

    #[test]
    fn de_explicit_child_name_ignores_auto() {
        // "database" node should NOT match when explicit name is "db"
        let v: ExplicitChildNameMatching = unison_kdl::from_str(
            r#"app {
                database "postgres://localhost"
            }"#,
        )
        .unwrap();
        assert!(v.database.is_none());
    }

    // --- 3e. unwrap_arg ---

    #[derive(Debug, PartialEq, KdlDeserialize)]
    #[kdl(name = "config")]
    struct UnwrapArg {
        #[kdl(child, unwrap_arg)]
        title: String,
        #[kdl(child, unwrap_arg)]
        description: Option<String>,
        #[kdl(child, unwrap_arg, default)]
        version: i64,
    }

    #[test]
    fn de_unwrap_arg_all_present() {
        let v: UnwrapArg = unison_kdl::from_str(
            r#"config {
                title "My App"
                description "A great app"
                version 3
            }"#,
        )
        .unwrap();
        assert_eq!(v.title, "My App");
        assert_eq!(v.description, Some("A great app".into()));
        assert_eq!(v.version, 3);
    }

    #[test]
    fn de_unwrap_arg_optional_absent() {
        let v: UnwrapArg = unison_kdl::from_str(
            r#"config {
                title "My App"
            }"#,
        )
        .unwrap();
        assert_eq!(v.title, "My App");
        assert_eq!(v.description, None);
        assert_eq!(v.version, 0); // default
    }

    // --- 3f. unwrap_args ---

    #[derive(Debug, PartialEq, KdlDeserialize)]
    #[kdl(name = "enum")]
    struct UnwrapArgs {
        #[kdl(argument)]
        name: String,
        #[kdl(child, unwrap_args)]
        values: Vec<String>,
    }

    #[test]
    fn de_unwrap_args() {
        let v: UnwrapArgs = unison_kdl::from_str(
            r#"enum "Status" {
                values "active" "inactive" "pending"
            }"#,
        )
        .unwrap();
        assert_eq!(v.values, vec!["active", "inactive", "pending"]);
    }

    #[test]
    fn de_unwrap_args_absent() {
        let v: UnwrapArgs = unison_kdl::from_str(r#"enum "Status""#).unwrap();
        assert!(v.values.is_empty());
    }
}

// ============================================================================
// 4. Children patterns (Vec<T>)
// ============================================================================

mod children {
    use super::*;

    #[derive(Debug, PartialEq, KdlDeserialize, KdlSerialize)]
    #[kdl(name = "step")]
    struct Step {
        #[kdl(argument)]
        name: String,
        #[kdl(property)]
        timeout: Option<i64>,
    }

    // --- 4a. Auto-name from child's #[kdl(name)] ---

    #[derive(Debug, PartialEq, KdlDeserialize, KdlSerialize)]
    #[kdl(name = "pipeline")]
    struct AutoNameChildren {
        #[kdl(argument)]
        name: String,
        #[kdl(children)]
        steps: Vec<Step>,
    }

    #[test]
    fn de_children_auto_name() {
        let v: AutoNameChildren = unison_kdl::from_str(
            r#"pipeline "deploy" {
                step "build" timeout=60
                step "test"
                step "push" timeout=120
            }"#,
        )
        .unwrap();
        assert_eq!(v.steps.len(), 3);
        assert_eq!(v.steps[0].name, "build");
        assert_eq!(v.steps[0].timeout, Some(60));
        assert_eq!(v.steps[1].timeout, None);
    }

    #[test]
    fn de_children_empty() {
        let v: AutoNameChildren = unison_kdl::from_str(r#"pipeline "empty""#).unwrap();
        assert!(v.steps.is_empty());
    }

    #[test]
    fn roundtrip_children() {
        let original = AutoNameChildren {
            name: "ci".into(),
            steps: vec![
                Step {
                    name: "lint".into(),
                    timeout: None,
                },
                Step {
                    name: "test".into(),
                    timeout: Some(300),
                },
            ],
        };
        let node = original.to_kdl_node().unwrap();
        let restored = AutoNameChildren::from_kdl_node(&node).unwrap();
        assert_eq!(original, restored);
    }

    // --- 4b. Explicit children name ---

    #[derive(Debug, PartialEq, KdlDeserialize)]
    #[kdl(name = "workflow")]
    struct ExplicitChildrenName {
        #[kdl(children(name = "step"))]
        phases: Vec<Step>,
    }

    #[test]
    fn de_children_explicit_name() {
        let v: ExplicitChildrenName = unison_kdl::from_str(
            r#"workflow {
                step "a"
                step "b"
            }"#,
        )
        .unwrap();
        assert_eq!(v.phases.len(), 2);
    }

    // --- 4c. Multiple children types ---

    #[derive(Debug, PartialEq, KdlDeserialize, KdlSerialize)]
    #[kdl(name = "port")]
    struct Port {
        #[kdl(property)]
        host: u16,
        #[kdl(property)]
        container: u16,
    }

    #[derive(Debug, PartialEq, KdlDeserialize, KdlSerialize)]
    #[kdl(name = "volume")]
    struct Volume {
        #[kdl(argument)]
        path: String,
    }

    #[derive(Debug, PartialEq, KdlDeserialize, KdlSerialize)]
    #[kdl(name = "service")]
    struct MultiChildTypes {
        #[kdl(argument)]
        name: String,
        #[kdl(children)]
        ports: Vec<Port>,
        #[kdl(children)]
        volumes: Vec<Volume>,
    }

    #[test]
    fn de_multiple_children_types() {
        let v: MultiChildTypes = unison_kdl::from_str(
            r#"service "web" {
                port host=80 container=80
                volume "/data"
                port host=443 container=443
                volume "/logs"
            }"#,
        )
        .unwrap();
        assert_eq!(v.ports.len(), 2);
        assert_eq!(v.volumes.len(), 2);
        assert_eq!(v.ports[0].host, 80);
        assert_eq!(v.volumes[1].path, "/logs");
    }

    #[test]
    fn roundtrip_multiple_children_types() {
        let original = MultiChildTypes {
            name: "api".into(),
            ports: vec![Port {
                host: 8080,
                container: 80,
            }],
            volumes: vec![Volume {
                path: "/app".into(),
            }],
        };
        let node = original.to_kdl_node().unwrap();
        let restored = MultiChildTypes::from_kdl_node(&node).unwrap();
        assert_eq!(original, restored);
    }
}

// ============================================================================
// 5. Child auto-name with #[kdl(name = "kebab-case")]
// ============================================================================

mod child_auto_name {
    use super::*;

    #[derive(Debug, PartialEq, KdlDeserialize)]
    #[kdl(name = "post-setup")]
    struct PostSetup {
        #[kdl(argument)]
        command: String,
    }

    #[derive(Debug, PartialEq, KdlDeserialize)]
    #[kdl(name = "pre-build")]
    struct PreBuild {
        #[kdl(arguments)]
        commands: Vec<String>,
    }

    #[derive(Debug, PartialEq, KdlDeserialize)]
    #[kdl(name = "on-error")]
    struct OnError {
        #[kdl(argument)]
        action: String,
    }

    // The core Issue #3 scenario: field name ≠ KDL node name
    #[derive(Debug, PartialEq, KdlDeserialize)]
    #[kdl(document)]
    struct HookConfig {
        #[kdl(child)]
        post_setup: Option<PostSetup>,
        #[kdl(child)]
        pre_build: Option<PreBuild>,
        #[kdl(children)]
        on_errors: Vec<OnError>,
    }

    #[test]
    fn de_kebab_case_auto_resolve() {
        let v: HookConfig = unison_kdl::from_str(
            r#"
            post-setup "bun install"
            pre-build "cargo" "build"
            on-error "notify"
            on-error "rollback"
        "#,
        )
        .unwrap();
        assert_eq!(v.post_setup.unwrap().command, "bun install");
        assert_eq!(v.pre_build.unwrap().commands, vec!["cargo", "build"]);
        assert_eq!(v.on_errors.len(), 2);
    }

    #[test]
    fn de_kebab_case_all_absent() {
        let v: HookConfig = unison_kdl::from_str("").unwrap();
        assert!(v.post_setup.is_none());
        assert!(v.pre_build.is_none());
        assert!(v.on_errors.is_empty());
    }

    // Verify that the OLD workaround also still works
    #[derive(Debug, PartialEq, KdlDeserialize)]
    #[kdl(document)]
    struct OldWorkaround {
        #[kdl(child(name = "post-setup"))]
        post_setup: Option<PostSetup>,
    }

    #[test]
    fn de_old_workaround_still_works() {
        let v: OldWorkaround =
            unison_kdl::from_str(r#"post-setup "npm install""#).unwrap();
        assert_eq!(v.post_setup.unwrap().command, "npm install");
    }
}

// ============================================================================
// 6. ChildMap patterns
// ============================================================================

mod child_map {
    use super::*;

    // --- 6a. With wrapper node ---

    #[derive(Debug, PartialEq, KdlDeserialize, KdlSerialize)]
    #[kdl(name = "service")]
    struct WithWrapper {
        #[kdl(argument)]
        name: String,
        #[kdl(child_map, name = "env")]
        environment: HashMap<String, String>,
    }

    #[test]
    fn de_child_map_with_wrapper() {
        let v: WithWrapper = unison_kdl::from_str(
            r#"service "api" {
                env {
                    PORT "8080"
                    HOST "0.0.0.0"
                }
            }"#,
        )
        .unwrap();
        assert_eq!(v.environment.get("PORT"), Some(&"8080".to_string()));
        assert_eq!(v.environment.get("HOST"), Some(&"0.0.0.0".to_string()));
    }

    #[test]
    fn de_child_map_wrapper_absent() {
        let v: WithWrapper = unison_kdl::from_str(r#"service "api""#).unwrap();
        assert!(v.environment.is_empty());
    }

    #[test]
    fn roundtrip_child_map() {
        let mut env = HashMap::new();
        env.insert("KEY".into(), "value".into());
        let original = WithWrapper {
            name: "svc".into(),
            environment: env,
        };
        let node = original.to_kdl_node().unwrap();
        let restored = WithWrapper::from_kdl_node(&node).unwrap();
        assert_eq!(original, restored);
    }

    // --- 6b. Without wrapper (direct children) ---

    #[derive(Debug, PartialEq, KdlDeserialize, KdlSerialize)]
    #[kdl(name = "labels")]
    struct DirectMap {
        #[kdl(child_map)]
        labels: HashMap<String, String>,
    }

    #[test]
    fn de_child_map_direct() {
        let v: DirectMap = unison_kdl::from_str(
            r#"labels {
                app "frontend"
                tier "web"
            }"#,
        )
        .unwrap();
        assert_eq!(v.labels.get("app"), Some(&"frontend".to_string()));
        assert_eq!(v.labels.get("tier"), Some(&"web".to_string()));
    }
}

// ============================================================================
// 7. Document-level deserialization
// ============================================================================

mod document {
    use super::*;

    #[derive(Debug, PartialEq, KdlDeserialize)]
    #[kdl(name = "route")]
    struct Route {
        #[kdl(argument)]
        path: String,
        #[kdl(property)]
        method: Option<String>,
    }

    #[derive(Debug, PartialEq, KdlDeserialize)]
    #[kdl(name = "middleware")]
    struct Middleware {
        #[kdl(argument)]
        name: String,
    }

    #[derive(Debug, PartialEq, KdlDeserialize)]
    #[kdl(document)]
    struct RouterConfig {
        #[kdl(children)]
        routes: Vec<Route>,
        #[kdl(children)]
        middlewares: Vec<Middleware>,
    }

    #[test]
    fn de_document_multiple_types() {
        let v: RouterConfig = unison_kdl::from_str(
            r#"
            middleware "auth"
            middleware "cors"
            route "/api/users" method="GET"
            route "/api/posts"
        "#,
        )
        .unwrap();
        assert_eq!(v.middlewares.len(), 2);
        assert_eq!(v.routes.len(), 2);
        assert_eq!(v.routes[0].method, Some("GET".into()));
        assert_eq!(v.routes[1].method, None);
    }

    #[test]
    fn de_document_empty() {
        let v: RouterConfig = unison_kdl::from_str("").unwrap();
        assert!(v.routes.is_empty());
        assert!(v.middlewares.is_empty());
    }
}

// ============================================================================
// 8. Enum scalar mapping
// ============================================================================

mod enums {
    use super::*;

    // --- 8a. With rename ---

    #[derive(Debug, PartialEq, Clone, KdlDeserialize, KdlSerialize)]
    enum LogLevel {
        #[kdl(rename = "debug")]
        Debug,
        #[kdl(rename = "info")]
        Info,
        #[kdl(rename = "warn")]
        Warn,
        #[kdl(rename = "error")]
        Error,
    }

    #[test]
    fn de_enum_all_variants() {
        for (s, expected) in [
            ("debug", LogLevel::Debug),
            ("info", LogLevel::Info),
            ("warn", LogLevel::Warn),
            ("error", LogLevel::Error),
        ] {
            let val = KdlValue::String(s.to_string());
            let v = LogLevel::from_kdl_value(&val).unwrap();
            assert_eq!(v, expected);
        }
    }

    #[test]
    fn de_enum_invalid() {
        let val = KdlValue::String("trace".into());
        assert!(LogLevel::from_kdl_value(&val).is_err());
    }

    // --- 8b. Without rename (auto snake_case) ---

    #[derive(Debug, PartialEq, Clone, KdlDeserialize, KdlSerialize)]
    enum BuildMode {
        Debug,
        Release,
        RelWithDebInfo,
    }

    #[test]
    fn de_enum_auto_snake_case() {
        let val = KdlValue::String("rel_with_deb_info".into());
        let v = BuildMode::from_kdl_value(&val).unwrap();
        assert_eq!(v, BuildMode::RelWithDebInfo);
    }

    // --- 8c. Enum as property ---

    #[derive(Debug, PartialEq, KdlDeserialize, KdlSerialize)]
    #[kdl(name = "build")]
    struct BuildConfig {
        #[kdl(property)]
        mode: LogLevel,
        #[kdl(property)]
        optional_mode: Option<LogLevel>,
    }

    #[test]
    fn de_enum_as_property() {
        let v: BuildConfig =
            unison_kdl::from_str(r#"build mode="info" optional_mode="warn""#).unwrap();
        assert_eq!(v.mode, LogLevel::Info);
        assert_eq!(v.optional_mode, Some(LogLevel::Warn));
    }

    #[test]
    fn roundtrip_enum_property() {
        let original = BuildConfig {
            mode: LogLevel::Error,
            optional_mode: Some(LogLevel::Debug),
        };
        let node = original.to_kdl_node().unwrap();
        let restored = BuildConfig::from_kdl_node(&node).unwrap();
        assert_eq!(original, restored);
    }
}

// ============================================================================
// 9. Skip field
// ============================================================================

mod skip {
    use super::*;

    #[derive(Debug, PartialEq, KdlDeserialize)]
    #[kdl(name = "item")]
    struct WithSkip {
        #[kdl(argument)]
        name: String,
        #[kdl(skip)]
        internal_id: u64,
    }

    #[test]
    fn de_skip_uses_default() {
        let v: WithSkip = unison_kdl::from_str(r#"item "test""#).unwrap();
        assert_eq!(v.name, "test");
        assert_eq!(v.internal_id, 0); // Default::default()
    }
}

// ============================================================================
// 10. All primitive types
// ============================================================================

mod primitives {
    use super::*;

    #[derive(Debug, PartialEq, KdlDeserialize, KdlSerialize)]
    #[kdl(name = "types")]
    struct AllTypes {
        #[kdl(property)]
        s: String,
        #[kdl(property)]
        i32_val: i32,
        #[kdl(property)]
        i64_val: i64,
        #[kdl(property)]
        u16_val: u16,
        #[kdl(property)]
        u32_val: u32,
        #[kdl(property)]
        u64_val: u64,
        #[kdl(property)]
        usize_val: usize,
        #[kdl(property)]
        f64_val: f64,
        #[kdl(property)]
        bool_val: bool,
        #[kdl(property)]
        path_val: PathBuf,
    }

    #[test]
    fn de_all_primitive_types() {
        let v: AllTypes = unison_kdl::from_str(
            r#"types s="hello" i32_val=42 i64_val=9999999 u16_val=65535 u32_val=100 u64_val=200 usize_val=300 f64_val=3.14 bool_val=#true path_val="/usr/bin""#,
        ).unwrap();
        assert_eq!(v.s, "hello");
        assert_eq!(v.i32_val, 42);
        assert_eq!(v.i64_val, 9999999);
        assert_eq!(v.u16_val, 65535);
        assert_eq!(v.u32_val, 100);
        assert_eq!(v.u64_val, 200);
        assert_eq!(v.usize_val, 300);
        assert_eq!(v.f64_val, 3.14);
        assert_eq!(v.bool_val, true);
        assert_eq!(v.path_val, PathBuf::from("/usr/bin"));
    }

    #[test]
    fn roundtrip_all_types() {
        let original = AllTypes {
            s: "test".into(),
            i32_val: -42,
            i64_val: i64::MAX,
            u16_val: 1000,
            u32_val: 2000,
            u64_val: 3000,
            usize_val: 4000,
            f64_val: 2.718,
            bool_val: false,
            path_val: PathBuf::from("/tmp/test"),
        };
        let node = original.to_kdl_node().unwrap();
        let restored = AllTypes::from_kdl_node(&node).unwrap();
        assert_eq!(original, restored);
    }
}

// ============================================================================
// 11. Deep nesting (3+ levels)
// ============================================================================

mod nesting {
    use super::*;

    #[derive(Debug, PartialEq, KdlDeserialize, KdlSerialize)]
    #[kdl(name = "field")]
    struct Field {
        #[kdl(argument)]
        name: String,
        #[kdl(property(rename = "type"))]
        field_type: String,
    }

    #[derive(Debug, PartialEq, KdlDeserialize, KdlSerialize)]
    #[kdl(name = "message")]
    struct Message {
        #[kdl(argument)]
        name: String,
        #[kdl(children)]
        fields: Vec<Field>,
    }

    #[derive(Debug, PartialEq, KdlDeserialize, KdlSerialize)]
    #[kdl(name = "protocol")]
    struct Protocol {
        #[kdl(argument)]
        name: String,
        #[kdl(children)]
        messages: Vec<Message>,
    }

    #[test]
    fn de_three_level_nesting() {
        let v: Protocol = unison_kdl::from_str(
            r#"protocol "MyProto" {
                message "Request" {
                    field "id" type="u64"
                    field "body" type="string"
                }
                message "Response" {
                    field "status" type="i32"
                }
            }"#,
        )
        .unwrap();
        assert_eq!(v.name, "MyProto");
        assert_eq!(v.messages.len(), 2);
        assert_eq!(v.messages[0].fields.len(), 2);
        assert_eq!(v.messages[0].fields[0].name, "id");
        assert_eq!(v.messages[0].fields[0].field_type, "u64");
        assert_eq!(v.messages[1].fields.len(), 1);
    }

    #[test]
    fn roundtrip_three_level() {
        let original = Protocol {
            name: "Test".into(),
            messages: vec![Message {
                name: "Ping".into(),
                fields: vec![Field {
                    name: "seq".into(),
                    field_type: "u32".into(),
                }],
            }],
        };
        let node = original.to_kdl_node().unwrap();
        let restored = Protocol::from_kdl_node(&node).unwrap();
        assert_eq!(original, restored);
    }
}

// ============================================================================
// 12. Mixed field kinds in one struct
// ============================================================================

mod mixed {
    use super::*;

    #[derive(Debug, PartialEq, KdlDeserialize, KdlSerialize)]
    #[kdl(name = "header")]
    struct Header {
        #[kdl(argument)]
        key: String,
        #[kdl(argument)]
        value: String,
    }

    #[derive(Debug, PartialEq, KdlDeserialize)]
    #[kdl(name = "endpoint")]
    struct Endpoint {
        // argument
        #[kdl(argument)]
        path: String,
        // property (required)
        #[kdl(property)]
        method: String,
        // property (optional)
        #[kdl(property)]
        timeout: Option<i64>,
        // child (optional, auto-name not applicable for unwrap_arg)
        #[kdl(child, unwrap_arg)]
        description: Option<String>,
        // children
        #[kdl(children)]
        headers: Vec<Header>,
    }

    #[test]
    fn de_all_field_kinds() {
        let v: Endpoint = unison_kdl::from_str(
            r#"endpoint "/api/users" method="POST" timeout=30 {
                description "Create a user"
                header "Content-Type" "application/json"
                header "Authorization" "Bearer token"
            }"#,
        )
        .unwrap();
        assert_eq!(v.path, "/api/users");
        assert_eq!(v.method, "POST");
        assert_eq!(v.timeout, Some(30));
        assert_eq!(v.description, Some("Create a user".into()));
        assert_eq!(v.headers.len(), 2);
    }

    #[test]
    fn de_mixed_minimal() {
        let v: Endpoint =
            unison_kdl::from_str(r#"endpoint "/health" method="GET""#).unwrap();
        assert_eq!(v.path, "/health");
        assert_eq!(v.timeout, None);
        assert_eq!(v.description, None);
        assert!(v.headers.is_empty());
    }

    // Note: Endpoint can't roundtrip because unwrap_arg fields
    // don't have KdlSerialize support yet. This is a known limitation.
}

// ============================================================================
// 13. Error cases
// ============================================================================

mod errors {
    use super::*;

    #[derive(Debug, PartialEq, KdlDeserialize)]
    #[kdl(name = "strict")]
    struct Strict {
        #[kdl(argument)]
        name: String,
        #[kdl(property)]
        required: i64,
    }

    #[test]
    fn err_missing_required_argument() {
        let result = unison_kdl::from_str::<Strict>(r#"strict"#);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("argument"), "error: {msg}");
    }

    #[test]
    fn err_missing_required_property() {
        let result = unison_kdl::from_str::<Strict>(r#"strict "ok""#);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("required"), "error: {msg}");
    }

    #[test]
    fn err_wrong_node_name() {
        let result = unison_kdl::from_str::<Strict>(r#"wrong "ok" required=1"#);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("unexpected") || msg.contains("strict"), "error: {msg}");
    }

    #[derive(Debug, PartialEq, KdlDeserialize)]
    #[kdl(name = "typed")]
    struct TypedField {
        #[kdl(property)]
        count: i64,
    }

    #[test]
    fn err_type_mismatch_string_as_int() {
        let result = unison_kdl::from_str::<TypedField>(r#"typed count="not_a_number""#);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("type mismatch") || msg.contains("integer"), "error: {msg}");
    }

    #[derive(Debug, PartialEq, KdlDeserialize)]
    #[kdl(name = "parent")]
    struct RequiredChild {
        #[kdl(child)]
        strict: Strict,
    }

    #[test]
    fn err_missing_required_child_auto_name() {
        let result = unison_kdl::from_str::<RequiredChild>(r#"parent"#);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("child") || msg.contains("strict"), "error: {msg}");
    }

    // Error context: struct name is included in error messages
    #[test]
    fn err_context_includes_struct_name() {
        let result = unison_kdl::from_str::<Strict>(r#"strict"#);
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("Strict"), "should contain struct name, got: {msg}");
    }

    #[test]
    fn err_context_nested() {
        let result = unison_kdl::from_str::<RequiredChild>(r#"parent"#);
        let msg = result.unwrap_err().to_string();
        // Should show nested context: "in RequiredChild: ..."
        assert!(msg.contains("RequiredChild"), "should contain parent struct, got: {msg}");
    }
}

// ============================================================================
// 14. Real-world scenario: Docker Compose-like config
// ============================================================================

mod real_world {
    use super::*;

    #[derive(Debug, PartialEq, Clone, KdlDeserialize, KdlSerialize)]
    enum RestartPolicy {
        #[kdl(rename = "no")]
        No,
        #[kdl(rename = "always")]
        Always,
        #[kdl(rename = "unless-stopped")]
        UnlessStopped,
        #[kdl(rename = "on-failure")]
        OnFailure,
    }

    #[derive(Debug, PartialEq, KdlDeserialize, KdlSerialize)]
    #[kdl(name = "port")]
    struct Port {
        #[kdl(property)]
        host: u16,
        #[kdl(property)]
        container: u16,
        #[kdl(property)]
        protocol: Option<String>,
    }

    #[derive(Debug, PartialEq, KdlDeserialize, KdlSerialize)]
    #[kdl(name = "volume")]
    struct Volume {
        #[kdl(property)]
        host: String,
        #[kdl(property)]
        container: String,
        #[kdl(property)]
        read_only: Option<bool>,
    }

    #[derive(Debug, PartialEq, KdlDeserialize, KdlSerialize)]
    #[kdl(name = "depends-on")]
    struct DependsOn {
        #[kdl(arguments)]
        services: Vec<String>,
    }

    #[derive(Debug, PartialEq, KdlDeserialize, KdlSerialize)]
    #[kdl(name = "service")]
    struct Service {
        #[kdl(argument)]
        name: String,
        #[kdl(property)]
        image: String,
        #[kdl(property)]
        restart: Option<RestartPolicy>,
        #[kdl(children)]
        ports: Vec<Port>,
        #[kdl(children)]
        volumes: Vec<Volume>,
        // auto-name resolves "depends-on" via DependsOn::kdl_node_name()
        #[kdl(child)]
        depends_on: Option<DependsOn>,
        #[kdl(child_map, name = "env")]
        environment: HashMap<String, String>,
    }

    #[derive(Debug, PartialEq, KdlDeserialize)]
    #[kdl(document)]
    struct ComposeFile {
        #[kdl(children)]
        services: Vec<Service>,
    }

    #[test]
    fn de_full_compose_like() {
        let v: ComposeFile = unison_kdl::from_str(
            r#"
            service "postgres" image="postgres:17" restart="always" {
                port host=5432 container=5432
                volume host="/data/pg" container="/var/lib/postgresql/data"
                env {
                    POSTGRES_PASSWORD "secret"
                    POSTGRES_DB "mydb"
                }
            }

            service "api" image="myapp:latest" restart="unless-stopped" {
                port host=8080 container=8080
                port host=9090 container=9090 protocol="grpc"
                depends-on "postgres"
                env {
                    DATABASE_URL "postgres://postgres:secret@postgres/mydb"
                }
            }

            service "nginx" image="nginx:alpine" {
                port host=80 container=80
                port host=443 container=443
                volume host="./nginx.conf" container="/etc/nginx/nginx.conf" read_only=#true
                depends-on "api"
            }
        "#,
        )
        .unwrap();

        assert_eq!(v.services.len(), 3);

        // postgres
        let pg = &v.services[0];
        assert_eq!(pg.name, "postgres");
        assert_eq!(pg.restart, Some(RestartPolicy::Always));
        assert_eq!(pg.ports.len(), 1);
        assert_eq!(pg.volumes.len(), 1);
        assert_eq!(
            pg.environment.get("POSTGRES_PASSWORD"),
            Some(&"secret".to_string())
        );
        assert!(pg.depends_on.is_none());

        // api
        let api = &v.services[1];
        assert_eq!(api.name, "api");
        assert_eq!(api.restart, Some(RestartPolicy::UnlessStopped));
        assert_eq!(api.ports.len(), 2);
        assert_eq!(api.ports[1].protocol, Some("grpc".into()));
        assert_eq!(
            api.depends_on.as_ref().unwrap().services,
            vec!["postgres"]
        );

        // nginx
        let nginx = &v.services[2];
        assert_eq!(nginx.volumes[0].read_only, Some(true));
        assert_eq!(
            nginx.depends_on.as_ref().unwrap().services,
            vec!["api"]
        );
    }

    #[test]
    fn roundtrip_service() {
        let mut env = HashMap::new();
        env.insert("KEY".into(), "val".into());
        let original = Service {
            name: "test".into(),
            image: "alpine:latest".into(),
            restart: Some(RestartPolicy::OnFailure),
            ports: vec![Port {
                host: 80,
                container: 80,
                protocol: None,
            }],
            volumes: vec![],
            depends_on: Some(DependsOn {
                services: vec!["db".into()],
            }),
            environment: env,
        };
        let node = original.to_kdl_node().unwrap();
        let restored = Service::from_kdl_node(&node).unwrap();
        assert_eq!(original, restored);
    }
}

// ============================================================================
// 15. Alias support
// ============================================================================

mod alias {
    use super::*;

    #[derive(Debug, PartialEq, KdlDeserialize)]
    #[kdl(name = "database", alias = "db")]
    struct Database {
        #[kdl(argument)]
        url: String,
    }

    #[test]
    fn de_primary_name() {
        let v: Database = unison_kdl::from_str(r#"database "pg://localhost""#).unwrap();
        assert_eq!(v.url, "pg://localhost");
    }

    #[test]
    fn de_alias_name() {
        let v: Database = unison_kdl::from_str(r#"db "pg://localhost""#).unwrap();
        assert_eq!(v.url, "pg://localhost");
    }

    #[test]
    fn de_wrong_name_still_errors() {
        let result = unison_kdl::from_str::<Database>(r#"datasource "pg://localhost""#);
        assert!(result.is_err());
    }

    // Multiple aliases
    #[derive(Debug, PartialEq, KdlDeserialize)]
    #[kdl(name = "database", alias = "db", alias = "datasource")]
    struct FlexibleDb {
        #[kdl(argument)]
        url: String,
    }

    #[test]
    fn de_multiple_aliases() {
        // primary
        let v: FlexibleDb = unison_kdl::from_str(r#"database "a""#).unwrap();
        assert_eq!(v.url, "a");
        // alias 1
        let v: FlexibleDb = unison_kdl::from_str(r#"db "b""#).unwrap();
        assert_eq!(v.url, "b");
        // alias 2
        let v: FlexibleDb = unison_kdl::from_str(r#"datasource "c""#).unwrap();
        assert_eq!(v.url, "c");
    }

    // Alias with child(name = "...") — the original conflict scenario
    #[derive(Debug, PartialEq, KdlDeserialize)]
    #[kdl(name = "app")]
    struct AppWithAlias {
        #[kdl(child(name = "db"))]
        database: Option<Database>,
    }

    #[test]
    fn de_child_explicit_name_matches_alias() {
        // parent searches for "db", Database accepts "db" as alias
        let v: AppWithAlias = unison_kdl::from_str(
            r#"app {
                db "postgres://localhost"
            }"#,
        )
        .unwrap();
        assert!(v.database.is_some());
        assert_eq!(v.database.unwrap().url, "postgres://localhost");
    }

    // Auto-name still uses primary name, not aliases
    #[derive(Debug, PartialEq, KdlDeserialize)]
    #[kdl(name = "app2")]
    struct AppAutoName {
        #[kdl(child)]
        database: Option<Database>,
    }

    #[test]
    fn de_auto_name_uses_primary() {
        // kdl_node_name() returns "database" (primary), not "db" (alias)
        let v: AppAutoName = unison_kdl::from_str(
            r#"app2 {
                database "pg://"
            }"#,
        )
        .unwrap();
        assert!(v.database.is_some());
    }

    #[test]
    fn de_auto_name_alias_not_searched() {
        // auto-name searches "database", not "db"
        let v: AppAutoName = unison_kdl::from_str(
            r#"app2 {
                db "pg://"
            }"#,
        )
        .unwrap();
        assert!(v.database.is_none());
    }
}

// ============================================================================
// 16. kdl_node_name() trait method
// ============================================================================

mod trait_method {
    use super::*;

    #[derive(Debug, PartialEq, KdlDeserialize)]
    #[kdl(name = "custom-name")]
    struct WithName {
        #[kdl(argument)]
        val: String,
    }

    #[derive(Debug, PartialEq, KdlDeserialize)]
    struct WithoutName {
        #[kdl(argument)]
        val: String,
    }

    #[test]
    fn kdl_node_name_returns_some_when_set() {
        assert_eq!(WithName::kdl_node_name(), Some("custom-name"));
    }

    #[test]
    fn kdl_node_name_returns_none_when_unset() {
        assert_eq!(WithoutName::kdl_node_name(), None);
    }
}

// ============================================================================
// 17. unwrap_arg / unwrap_args serialize (roundtrip)
// ============================================================================

mod unwrap_serialize {
    use super::*;

    #[derive(Debug, PartialEq, KdlDeserialize, KdlSerialize)]
    #[kdl(name = "config")]
    struct UnwrapArgRoundtrip {
        #[kdl(child, unwrap_arg)]
        title: String,
        #[kdl(child, unwrap_arg)]
        description: Option<String>,
        #[kdl(child, unwrap_arg, default)]
        version: i64,
    }

    #[test]
    fn roundtrip_unwrap_arg_all() {
        let original = UnwrapArgRoundtrip {
            title: "My App".into(),
            description: Some("A great app".into()),
            version: 3,
        };
        let node = original.to_kdl_node().unwrap();
        let restored = UnwrapArgRoundtrip::from_kdl_node(&node).unwrap();
        assert_eq!(original, restored);
    }

    #[test]
    fn roundtrip_unwrap_arg_optional_none() {
        let original = UnwrapArgRoundtrip {
            title: "Minimal".into(),
            description: None,
            version: 0,
        };
        let node = original.to_kdl_node().unwrap();
        let restored = UnwrapArgRoundtrip::from_kdl_node(&node).unwrap();
        assert_eq!(restored.title, "Minimal");
        assert_eq!(restored.description, None);
    }

    #[derive(Debug, PartialEq, KdlDeserialize, KdlSerialize)]
    #[kdl(name = "enum")]
    struct UnwrapArgsRoundtrip {
        #[kdl(argument)]
        name: String,
        #[kdl(child, unwrap_args)]
        values: Vec<String>,
    }

    #[test]
    fn roundtrip_unwrap_args() {
        let original = UnwrapArgsRoundtrip {
            name: "Status".into(),
            values: vec!["active".into(), "inactive".into()],
        };
        let node = original.to_kdl_node().unwrap();
        let restored = UnwrapArgsRoundtrip::from_kdl_node(&node).unwrap();
        assert_eq!(original, restored);
    }

    #[test]
    fn roundtrip_unwrap_args_empty() {
        let original = UnwrapArgsRoundtrip {
            name: "Empty".into(),
            values: vec![],
        };
        let node = original.to_kdl_node().unwrap();
        let restored = UnwrapArgsRoundtrip::from_kdl_node(&node).unwrap();
        assert_eq!(original, restored);
    }
}

// ============================================================================
// 18. Document serialize (roundtrip)
// ============================================================================

mod document_serialize {
    use super::*;

    #[derive(Debug, PartialEq, KdlDeserialize, KdlSerialize)]
    #[kdl(name = "route")]
    struct Route {
        #[kdl(argument)]
        path: String,
        #[kdl(property)]
        method: Option<String>,
    }

    #[derive(Debug, PartialEq, KdlDeserialize, KdlSerialize)]
    #[kdl(name = "middleware")]
    struct Middleware {
        #[kdl(argument)]
        name: String,
    }

    #[derive(Debug, PartialEq, KdlDeserialize, KdlSerialize)]
    #[kdl(document)]
    struct RouterConfig {
        #[kdl(children)]
        routes: Vec<Route>,
        #[kdl(children)]
        middlewares: Vec<Middleware>,
    }

    #[test]
    fn roundtrip_document() {
        let original = RouterConfig {
            routes: vec![
                Route { path: "/api".into(), method: Some("GET".into()) },
                Route { path: "/health".into(), method: None },
            ],
            middlewares: vec![
                Middleware { name: "auth".into() },
            ],
        };
        let doc = original.to_kdl_doc().unwrap();
        // Document should have 3 top-level nodes
        assert_eq!(doc.nodes().len(), 3);

        let restored: RouterConfig = unison_kdl::from_doc(&doc).unwrap();
        assert_eq!(original, restored);
    }

    #[test]
    fn document_to_string_roundtrip() {
        let original = RouterConfig {
            routes: vec![Route { path: "/".into(), method: None }],
            middlewares: vec![],
        };
        let kdl_string = unison_kdl::to_string(&original).unwrap();
        assert!(kdl_string.contains("route"));
        let restored: RouterConfig = unison_kdl::from_str(&kdl_string).unwrap();
        assert_eq!(original, restored);
    }

    #[test]
    fn document_empty_roundtrip() {
        let original = RouterConfig {
            routes: vec![],
            middlewares: vec![],
        };
        let doc = original.to_kdl_doc().unwrap();
        assert_eq!(doc.nodes().len(), 0);
        let restored: RouterConfig = unison_kdl::from_doc(&doc).unwrap();
        assert_eq!(original, restored);
    }
}

// ============================================================================
// 19. Flatten
// ============================================================================

mod flatten {
    use super::*;

    #[derive(Debug, PartialEq, KdlDeserialize, KdlSerialize)]
    struct Meta {
        #[kdl(property)]
        description: Option<String>,
        #[kdl(property)]
        deprecated: Option<bool>,
    }

    #[derive(Debug, PartialEq, KdlDeserialize, KdlSerialize)]
    #[kdl(name = "service")]
    struct ServiceWithMeta {
        #[kdl(argument)]
        name: String,
        #[kdl(property)]
        image: String,
        #[kdl(flatten)]
        meta: Meta,
    }

    #[test]
    fn de_flatten() {
        let v: ServiceWithMeta = unison_kdl::from_str(
            r#"service "api" image="nginx" description="Main API" deprecated=#true"#,
        ).unwrap();
        assert_eq!(v.name, "api");
        assert_eq!(v.image, "nginx");
        assert_eq!(v.meta.description, Some("Main API".into()));
        assert_eq!(v.meta.deprecated, Some(true));
    }

    #[test]
    fn de_flatten_partial() {
        let v: ServiceWithMeta = unison_kdl::from_str(
            r#"service "api" image="nginx""#,
        ).unwrap();
        assert_eq!(v.meta.description, None);
        assert_eq!(v.meta.deprecated, None);
    }

    #[test]
    fn roundtrip_flatten() {
        let original = ServiceWithMeta {
            name: "web".into(),
            image: "alpine".into(),
            meta: Meta {
                description: Some("Web server".into()),
                deprecated: Some(false),
            },
        };
        let node = original.to_kdl_node().unwrap();

        // Verify flattened properties are on the node directly
        use unison_kdl::KdlNodeExt;
        assert_eq!(node.prop("description").and_then(|v| v.as_string()), Some("Web server"));
        assert_eq!(node.prop("deprecated").and_then(|v| v.as_bool()), Some(false));

        let restored = ServiceWithMeta::from_kdl_node(&node).unwrap();
        assert_eq!(original, restored);
    }

    // Flatten with children
    #[derive(Debug, PartialEq, KdlDeserialize, KdlSerialize)]
    #[kdl(name = "port")]
    struct FlatPort {
        #[kdl(property)]
        host: u16,
        #[kdl(property)]
        container: u16,
    }

    #[derive(Debug, PartialEq, KdlDeserialize, KdlSerialize)]
    struct Networking {
        #[kdl(children)]
        ports: Vec<FlatPort>,
    }

    #[derive(Debug, PartialEq, KdlDeserialize, KdlSerialize)]
    #[kdl(name = "app")]
    struct AppWithFlatten {
        #[kdl(argument)]
        name: String,
        #[kdl(flatten)]
        networking: Networking,
    }

    #[test]
    fn roundtrip_flatten_with_children() {
        let original = AppWithFlatten {
            name: "myapp".into(),
            networking: Networking {
                ports: vec![
                    FlatPort { host: 80, container: 80 },
                    FlatPort { host: 443, container: 443 },
                ],
            },
        };
        let node = original.to_kdl_node().unwrap();
        let restored = AppWithFlatten::from_kdl_node(&node).unwrap();
        assert_eq!(original, restored);
    }
}

// ============================================================================
// 18. Enum data variants
// ============================================================================

mod enum_data_variants {
    use super::*;

    // -- Struct variant: fields with #[kdl(argument)], #[kdl(property)] --

    #[derive(Debug, PartialEq, KdlDeserialize, KdlSerialize)]
    enum Command {
        #[kdl(rename = "move")]
        Move {
            #[kdl(property)]
            x: i64,
            #[kdl(property)]
            y: i64,
        },
        Resize {
            #[kdl(argument)]
            width: i64,
            #[kdl(argument)]
            height: i64,
        },
        Quit,
    }

    #[test]
    fn de_struct_variant_properties() {
        let doc: kdl::KdlDocument = r#"move x=10 y=20"#.parse().unwrap();
        let node = doc.nodes().first().unwrap();
        let cmd = Command::from_kdl_node(node).unwrap();
        assert_eq!(cmd, Command::Move { x: 10, y: 20 });
    }

    #[test]
    fn de_struct_variant_arguments() {
        let doc: kdl::KdlDocument = r#"resize 800 600"#.parse().unwrap();
        let node = doc.nodes().first().unwrap();
        let cmd = Command::from_kdl_node(node).unwrap();
        assert_eq!(cmd, Command::Resize { width: 800, height: 600 });
    }

    #[test]
    fn de_unit_variant_in_data_enum() {
        let doc: kdl::KdlDocument = "quit".parse().unwrap();
        let node = doc.nodes().first().unwrap();
        let cmd = Command::from_kdl_node(node).unwrap();
        assert_eq!(cmd, Command::Quit);
    }

    #[test]
    fn de_unknown_variant_errors() {
        let doc: kdl::KdlDocument = "unknown".parse().unwrap();
        let node = doc.nodes().first().unwrap();
        let result = Command::from_kdl_node(node);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("unknown variant 'unknown'"));
    }

    #[test]
    fn ser_struct_variant_properties() {
        let cmd = Command::Move { x: 10, y: 20 };
        let node = cmd.to_kdl_node().unwrap();
        assert_eq!(node.name().value(), "move");
        // Properties are serialized as key=value entries
        assert_eq!(node.entries().len(), 2);
    }

    #[test]
    fn ser_struct_variant_arguments() {
        let cmd = Command::Resize { width: 800, height: 600 };
        let node = cmd.to_kdl_node().unwrap();
        assert_eq!(node.name().value(), "resize");
        // Two positional arguments
        assert_eq!(node.entries().len(), 2);
    }

    #[test]
    fn ser_unit_variant_in_data_enum() {
        let cmd = Command::Quit;
        let node = cmd.to_kdl_node().unwrap();
        assert_eq!(node.name().value(), "quit");
        assert!(node.entries().is_empty());
    }

    #[test]
    fn roundtrip_struct_variant() {
        let original = Command::Move { x: -5, y: 42 };
        let node = original.to_kdl_node().unwrap();
        let restored = Command::from_kdl_node(&node).unwrap();
        assert_eq!(original, restored);
    }

    #[test]
    fn roundtrip_unit_variant_in_data_enum() {
        let original = Command::Quit;
        let node = original.to_kdl_node().unwrap();
        let restored = Command::from_kdl_node(&node).unwrap();
        assert_eq!(original, restored);
    }

    // -- Newtype variant --

    #[derive(Debug, PartialEq, KdlDeserialize, KdlSerialize)]
    #[kdl(name = "config")]
    struct InnerConfig {
        #[kdl(property)]
        debug: bool,
        #[kdl(property, default)]
        level: i64,
    }

    #[derive(Debug, PartialEq, KdlDeserialize, KdlSerialize)]
    enum Action {
        Start,
        Configure(InnerConfig),
        Stop,
    }

    #[test]
    fn de_newtype_variant() {
        let doc: kdl::KdlDocument = "configure debug=#true level=3".parse().unwrap();
        let node = doc.nodes().first().unwrap();
        let action = Action::from_kdl_node(node).unwrap();
        assert_eq!(action, Action::Configure(InnerConfig { debug: true, level: 3 }));
    }

    #[test]
    fn de_unit_variant_mixed_enum() {
        let doc: kdl::KdlDocument = "start".parse().unwrap();
        let node = doc.nodes().first().unwrap();
        let action = Action::from_kdl_node(node).unwrap();
        assert_eq!(action, Action::Start);
    }

    #[test]
    fn ser_newtype_variant() {
        let action = Action::Configure(InnerConfig { debug: false, level: 5 });
        let node = action.to_kdl_node().unwrap();
        // Node name should be overridden to "configure" (variant name), not "config" (inner struct name)
        assert_eq!(node.name().value(), "configure");
    }

    #[test]
    fn roundtrip_newtype_variant() {
        let original = Action::Configure(InnerConfig { debug: true, level: 7 });
        let node = original.to_kdl_node().unwrap();
        let restored = Action::from_kdl_node(&node).unwrap();
        assert_eq!(original, restored);
    }

    // -- Struct variant with child nodes --

    #[derive(Debug, PartialEq, KdlDeserialize, KdlSerialize)]
    #[kdl(name = "target")]
    struct DeployTarget {
        #[kdl(argument)]
        name: String,
    }

    #[derive(Debug, PartialEq, KdlDeserialize, KdlSerialize)]
    enum Step {
        Build {
            #[kdl(argument)]
            path: String,
            #[kdl(property, default)]
            release: bool,
        },
        Deploy {
            #[kdl(argument)]
            env: String,
            #[kdl(children)]
            targets: Vec<DeployTarget>,
        },
        Clean,
    }

    #[test]
    fn de_struct_variant_with_children() {
        let kdl = r#"deploy "production" {
            target "web-01"
            target "web-02"
        }"#;
        let doc: kdl::KdlDocument = kdl.parse().unwrap();
        let node = doc.nodes().first().unwrap();
        let step = Step::from_kdl_node(node).unwrap();
        assert_eq!(step, Step::Deploy {
            env: "production".to_string(),
            targets: vec![
                DeployTarget { name: "web-01".to_string() },
                DeployTarget { name: "web-02".to_string() },
            ],
        });
    }

    #[test]
    fn roundtrip_struct_variant_with_children() {
        let original = Step::Deploy {
            env: "staging".to_string(),
            targets: vec![DeployTarget { name: "app-01".to_string() }],
        };
        let node = original.to_kdl_node().unwrap();
        let restored = Step::from_kdl_node(&node).unwrap();
        assert_eq!(original, restored);
    }

    #[test]
    fn roundtrip_struct_variant_build() {
        let original = Step::Build {
            path: "./src".to_string(),
            release: true,
        };
        let node = original.to_kdl_node().unwrap();
        let restored = Step::from_kdl_node(&node).unwrap();
        assert_eq!(original, restored);
    }

    // -- Data enum as Vec<T> children of a parent struct --

    #[derive(Debug, PartialEq, KdlDeserialize, KdlSerialize)]
    #[kdl(name = "pipeline")]
    struct Pipeline {
        #[kdl(argument)]
        name: String,
        #[kdl(children)]
        steps: Vec<Step>,
    }

    #[test]
    fn de_vec_data_enum_children() {
        let kdl = r#"pipeline "deploy-flow" {
            build "./src" release=#true
            deploy "production" {
                target "web-01"
                target "web-02"
            }
            clean
        }"#;
        let doc: kdl::KdlDocument = kdl.parse().unwrap();
        let node = doc.nodes().first().unwrap();
        let pipeline = Pipeline::from_kdl_node(node).unwrap();
        assert_eq!(pipeline.name, "deploy-flow");
        assert_eq!(pipeline.steps.len(), 3);
        assert_eq!(pipeline.steps[0], Step::Build {
            path: "./src".to_string(),
            release: true,
        });
        assert_eq!(pipeline.steps[2], Step::Clean);
    }

    #[test]
    fn roundtrip_vec_data_enum_children() {
        let original = Pipeline {
            name: "ci".to_string(),
            steps: vec![
                Step::Build { path: ".".to_string(), release: false },
                Step::Deploy {
                    env: "staging".to_string(),
                    targets: vec![DeployTarget { name: "app-01".to_string() }],
                },
                Step::Clean,
            ],
        };
        let node = original.to_kdl_node().unwrap();
        let restored = Pipeline::from_kdl_node(&node).unwrap();
        assert_eq!(original, restored);
    }

    // -- Error context on data enum --

    #[test]
    fn err_context_on_struct_variant() {
        let doc: kdl::KdlDocument = r#"move x="not_a_number" y=20"#.parse().unwrap();
        let node = doc.nodes().first().unwrap();
        let result = Command::from_kdl_node(node);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        // Should include variant context
        assert!(err.contains("Command::Move"), "error was: {}", err);
    }

    // -- Struct variant with optional fields --

    #[derive(Debug, PartialEq, KdlDeserialize, KdlSerialize)]
    enum Event {
        Click {
            #[kdl(property)]
            x: i64,
            #[kdl(property)]
            y: i64,
            #[kdl(property)]
            button: Option<String>,
        },
        Keypress {
            #[kdl(argument)]
            key: String,
            #[kdl(property, default)]
            modifiers: i64,
        },
    }

    #[test]
    fn de_optional_field_present() {
        let doc: kdl::KdlDocument = r#"click x=100 y=200 button="left""#.parse().unwrap();
        let node = doc.nodes().first().unwrap();
        let event = Event::from_kdl_node(node).unwrap();
        assert_eq!(event, Event::Click { x: 100, y: 200, button: Some("left".to_string()) });
    }

    #[test]
    fn de_optional_field_absent() {
        let doc: kdl::KdlDocument = r#"click x=100 y=200"#.parse().unwrap();
        let node = doc.nodes().first().unwrap();
        let event = Event::from_kdl_node(node).unwrap();
        assert_eq!(event, Event::Click { x: 100, y: 200, button: None });
    }

    #[test]
    fn roundtrip_optional_present() {
        let original = Event::Click { x: 50, y: 75, button: Some("right".to_string()) };
        let node = original.to_kdl_node().unwrap();
        let restored = Event::from_kdl_node(&node).unwrap();
        assert_eq!(original, restored);
    }

    #[test]
    fn roundtrip_optional_absent() {
        let original = Event::Click { x: 50, y: 75, button: None };
        let node = original.to_kdl_node().unwrap();
        let restored = Event::from_kdl_node(&node).unwrap();
        assert_eq!(original, restored);
    }

    #[test]
    fn roundtrip_default_field() {
        let original = Event::Keypress { key: "a".to_string(), modifiers: 0 };
        let node = original.to_kdl_node().unwrap();
        let restored = Event::from_kdl_node(&node).unwrap();
        assert_eq!(original, restored);
    }
}
