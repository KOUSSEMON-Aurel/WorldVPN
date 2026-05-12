fn main() {
    // Tell Cargo that if any file in the migrations directory changes, 
    // it needs to re-run this build script and re-compile the crate.
    // This ensures sqlx::migrate!() macro is always up to date.
    println!("cargo:rerun-if-changed=migrations/");
}
