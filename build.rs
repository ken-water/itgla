fn main() -> Result<(), slint_build::CompileError> {
    println!("cargo:rerun-if-changed=assets/itgla.ico");
    println!("cargo:rerun-if-changed=assets/itgla.png");
    let emit_debug_info = std::env::var("PROFILE").is_ok_and(|profile| profile != "release");
    slint_build::compile_with_config(
        "ui/app.slint",
        slint_build::CompilerConfiguration::new().with_debug_info(emit_debug_info),
    )
}
