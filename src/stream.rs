//! NDKDL — KDL を 1 ノード = 1 レコードのストリームとして扱う。
//!
//! ログ・メトリクス・イベントストリーム向けのヘルパー。
//! [`crate::to_string_pretty`] がドキュメント全体の round-trip 用なのに対し、
//! こちらはファイル全体を読まずに **1 ノードを末尾へ追記**する。
//!
//! KDL spec 上、 トップレベルノードを連続して並べたものは valid な KDL
//! ドキュメントなので、 追記後のファイルは `#[kdl(document)]` 構造体で
//! そのまま読み戻せる。
//!
//! ```no_run
//! use club_kdl::{KdlSerialize, append_node};
//!
//! #[derive(KdlSerialize)]
//! #[kdl(name = "event")]
//! struct Event {
//!     #[kdl(argument)]
//!     kind: String,
//!     #[kdl(property)]
//!     seq: u32,
//! }
//!
//! append_node("events.kdl", &Event { kind: "start".into(), seq: 1 }).unwrap();
//! ```

use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

use crate::error::{Error, Result};
use crate::ser::KdlSerialize;

/// 値を 1 つの KDL ノードとしてファイルの末尾に追記する。
///
/// ファイルが存在しなければ作成する。 各ノードは改行で区切られ、
/// 追記後のファイルは複数トップレベルノードを持つ valid な KDL
/// ドキュメントになる。
///
/// # Errors
///
/// シリアライズに失敗した場合、 またはファイルのオープン / 書き込みに
/// 失敗した場合にエラーを返す (io エラーはパス情報を添えて [`Error::Custom`]
/// に包まれる)。
pub fn append_node<T: KdlSerialize>(path: impl AsRef<Path>, value: &T) -> Result<()> {
    let path = path.as_ref();
    let node = value.to_kdl_node()?;

    let mut line = node.to_string();
    if !line.ends_with('\n') {
        line.push('\n');
    }

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| Error::custom(format!("failed to open {}: {e}", path.display())))?;
    file.write_all(line.as_bytes())
        .map_err(|e| Error::custom(format!("failed to append to {}: {e}", path.display())))?;

    Ok(())
}
