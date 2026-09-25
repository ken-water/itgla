# Windows package

ITGLA's Windows x86_64 release is a portable package. Extract the ZIP to a writable directory and run `itgla.exe`. Windows may show a SmartScreen warning because the executable is not currently code-signed.

Application data is stored under the Windows local application-data directory. Preserve that directory before replacing or removing an installation.

## Build from Linux

Install Rust stable, the `x86_64-pc-windows-gnu` Rust target, MinGW-w64, Python 3, `zip`, and standard checksum and binary-inspection tools. On Debian or Ubuntu:

```bash
rustup target add x86_64-pc-windows-gnu
sudo apt-get install gcc-mingw-w64-x86-64 binutils-mingw-w64-x86-64 python3 zip file
./scripts/package-windows.sh
```

The packaging script performs a locked release build, strips the copied executable, verifies that it is a 64-bit Windows GUI PE binary, creates `dist/itgla-v<version>-windows-x86_64.zip`, and writes a sibling `.sha256` file.

The Linux cross-build verifies compilation, linking, package contents, and executable metadata. It does not replace launch and workflow testing on a supported Windows host.
