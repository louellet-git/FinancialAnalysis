//! Embeds the commit and the state of the code at the build time: the run log of `fa portfolio backtest` names them, so a
//! run with an old binary shows. No dependency.
use std::process::Command;

fn git(args: &[&str]) -> Option<String> {
    let o = Command::new("git").args(args).output().ok()?;
    o.status.success().then(|| String::from_utf8_lossy(&o.stdout).trim().to_string())
}

fn main() {
    let root = git(&["rev-parse", "--show-toplevel"]).unwrap_or_else(|| "..".to_string());
    let in_root = |args: &[&str]| -> Option<String> {
        let mut a = vec!["-C", root.as_str()];
        a.extend_from_slice(args);
        git(&a)
    };
    let commit = in_root(&["rev-parse", "HEAD"]).unwrap_or_default();
    // The same rule as the run: the whole repository, but not the run logs (a run log is not code).
    let dirty = in_root(&["status", "--porcelain", "--", ".", ":(exclude)trading/runs"]).is_none_or(|s| !s.is_empty());
    println!("cargo:rustc-env=FA_BUILD_COMMIT={commit}");
    println!("cargo:rustc-env=FA_BUILD_DIRTY={dirty}");
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../financial_analysis");
    for p in ["HEAD", "logs/HEAD"] {
        if let Some(path) = in_root(&["rev-parse", "--git-path", p]) {
            let path = if std::path::Path::new(&path).is_absolute() { path } else { format!("{root}/{path}") };
            println!("cargo:rerun-if-changed={path}");
        }
    }
}
