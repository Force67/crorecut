use std::process::Command;

fn main() {
    #[cfg(target_os = "windows")]
    {
        println!("cargo:rerun-if-changed=assets/crorecut.ico");
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/crorecut.ico");
        res.compile().expect("Failed to compile Windows resources");
    }

    embed_git_info();
}

/// Exposes `CRORECUT_GIT_HASH` and `CRORECUT_GIT_DATE` to the crate, read via `option_env!`
fn embed_git_info() {
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs/heads");
    println!("cargo:rerun-if-changed=.git/packed-refs");

    if let Some(hash) = git(&["rev-parse", "--short", "HEAD"]) {
        println!("cargo:rustc-env=CRORECUT_GIT_HASH={hash}");
    }
    // e.g. "7 Oct 2026". `%-d` would avoid the trim, but crashes git on Windows outside Git Bash
    if let Some(date) = git(&["log", "-1", "--format=%cd", "--date=format:%d %b %Y"]) {
        println!(
            "cargo:rustc-env=CRORECUT_GIT_DATE={}",
            date.trim_start_matches('0')
        );
    }
}

fn git(args: &[&str]) -> Option<String> {
    let output = Command::new("git").args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8(output.stdout).ok()?.trim().to_string();
    (!value.is_empty()).then_some(value)
}
