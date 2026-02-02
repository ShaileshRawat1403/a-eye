use crate::aeye::commands::plan::Intent;
use crate::aeye::commands::plan::Plan;
use crate::aeye::scanner::SystemProfile;
use anyhow::Result;
use std::path::Path;

pub async fn generate_patch_from_llm(
    _repo_root: &Path,
    _intent: &Intent,
    _system_profile: &SystemProfile,
    _plan: &Plan,
) -> Result<String> {
    Ok(r#"--- a/src/main.rs
+++ b/src/main.rs
@@ -1,5 +1,5 @@
 fn main() {
-    println!("Hello, world!");
+    println!("Hello from A-Eye!");
 }
"#
    .to_string())
}
