fn main() -> Result<(), slint_build::CompileError> {
    println!("cargo:rerun-if-changed=assets/itgla.ico");
    slint_build::compile("ui/app.slint")
}
