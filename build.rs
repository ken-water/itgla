fn main() -> Result<(), slint_build::CompileError> {
    println!("cargo:rerun-if-changed=assets/itgla.ico");
    println!("cargo:rerun-if-changed=assets/itgla.png");
    slint_build::compile("ui/app.slint")
}
