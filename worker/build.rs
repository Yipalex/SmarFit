fn main() {
    println!("cargo:rerun-if-env-changed=SMARFIT_COMMIT");
    let commit = std::env::var("SMARFIT_COMMIT").unwrap_or_else(|_| "local".into());
    println!("cargo:rustc-env=SMARFIT_BUILD_COMMIT={commit}");
}
