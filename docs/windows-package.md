# Windows installation and portable ZIP

Choose the setup EXE for a per-user installation. Administrator rights are not required. Setup creates Start Menu entries; the desktop shortcut is optional. The ZIP is an alternative: extract it and run `itgla.exe`.

## Data, upgrades and removal

Both distributions use `%APPDATA%\itgla` by default, including `itgla.db` and backups. The ZIP does not keep a separate database beside the executable. `ITGLA_DATA_DIR` can select another workspace directory.

Setup reuses the registered installation directory, refuses downgrades, and checks whether the target executable is in use before changing program files. If copying program files fails, setup attempts to restore the previous program files. If restoration fails, recovery copies remain in `.itgla-setup-backup` inside the installation directory; further installation is blocked until that directory is reviewed. Restore its known program files with ITGLA closed, then archive/remove that recovery directory before retrying. This is not a database rollback: installation itself never opens or migrates a workspace. Database migrations occur when the application opens a workspace; back up the workspace before upgrading.

Uninstall removes known program files and shortcuts only. Workspaces, settings and backups are retained. Unknown files in the installation directory are also retained. To remove personal data, close ITGLA and review/back up the workspace before deleting it manually.

For unattended deployment, use `setup.exe /S`. An optional directory override must be the final argument: `setup.exe /S /D=C:\Path With Spaces\ITGLA`. Downgrade and occupied-executable failures return nonzero. The registered quiet uninstall command uses `Uninstall.exe /S`. Validate deployment in a disposable account before using these commands on a real workspace.

## Build from Linux

Install Rust stable, the `x86_64-pc-windows-gnu` target, MinGW-w64, NSIS, ImageMagick, Python 3 with Pillow, DejaVu Sans, ZIP, and binary-inspection tools:

```bash
rustup target add x86_64-pc-windows-gnu
sudo apt-get install gcc-mingw-w64-x86-64 binutils-mingw-w64-x86-64 python3 python3-pil fonts-dejavu-core zip file nsis imagemagick
./scripts/package-windows.sh
```

`ITGLA_PACKAGE_OUTPUT` can select a separate candidate output directory. The script performs a locked release build, embeds icon/version metadata, generates branded high-DPI artwork, creates a solid-LZMA NSIS installer and a portable ZIP with SHA-256 sidecars. Candidate builds must not overwrite an already published release.

Unsigned builds may trigger SmartScreen. A checksum verifies file integrity; it does not replace publisher signing. See `docs/package-signing.md` for the separate signing stage.

Cross-compilation and package inspection do not replace Windows fresh-install, upgrade, uninstall and launch acceptance on supported Windows hosts.
