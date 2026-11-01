# Installer optimization delivery ? 2026-11-01

Implemented in the agreed order: lifecycle/data safety, Linux compatibility, installer appearance, compression and signing preparation. Local Windows/Linux candidates are under `dist/installer-candidate-20261101/`. These remain unsigned development candidates with embedded version 0.6.1, not replacements for the public release. No GitHub workflow was triggered, no public artifact was overwritten and no commit/push/release was performed.

## 1. Installation, upgrades and data retention

The Windows NSIS template is now maintained in `packaging/windows/installer.nsi`, rather than a large shell heredoc. It reuses the registered installation path, checks write access to a running mapped executable, serializes installer/uninstaller activity with a mutex, rejects downgrades (including silent attempts), stages payload files before installation, backs up known program files, restores them on copy failure, and retains durable `.itgla-setup-backup` copies when restoration itself fails. A preexisting recovery directory blocks another install instead of overwriting recovery evidence. This is file-level recovery; it does not promise a transaction across power loss, registry and shortcut failures.

Install and uninstall remain per-user without UAC elevation. Start Menu entries remain; desktop shortcut is now optional. Uninstall is nonrecursive and removes only known program files, keeping unknown files and workspaces. Uninstall registry entries now have quoted commands, a quiet uninstall command, homepage/help URL and installed-size metadata. Size is calculated after removing transient recovery copies.

`docs/windows-package.md` now documents the actual default `%APPDATA%\itgla`, the `ITGLA_DATA_DIR` override, workspace retention, the fact that the ZIP uses the same user workspace by default, silent arguments and recovery behavior. The installer never opens the database. Database migration happens at application launch, so a backup is still required before upgrading a real workspace.

## 2. Linux compatibility

`scripts/build-linux-baseline.sh` compiles in an isolated Ubuntu 22.04 container with locked offline Cargo dependencies. The tested image was `docker.1ms.run/library/ubuntu:22.04` with resolved manifest digest `sha256:076e7bf82319619f28dd0aa9c02ab34b055ad37100395b4f49a2ffba04fb5e11`; exact image ID is recorded in the smoke evidence. The original host-native binary requires glibc 2.39; the baseline candidate requires glibc 2.35. `scripts/check-linux-abi.py` rejects a newer ABI before packaging and checks every versioned ELF in the generated AppImage. The original 2.39 binary was actually rejected by this gate.

DEB dependencies are derived from the actual ELF using dpkg-shlibdeps. Explicit display dependencies cover libraries Winit loads dynamically, including libxkbcommon-x11. Maintainer, description and desktop-portal recommendations are populated. RPM keeps automatic ELF requirements and adds corresponding dynamically loaded X11 libraries. An optional prebuilt baseline binary can be selected for packaging in a dedicated checkout. The fallback that reuses an AppImage runtime now requires a matching checksum sidecar.

A clean Ubuntu 22.04 container installed the candidate DEB and opened a real ITGLA X11 window under Xvfb; the screenshot was inspected. The first clean launch caught a missing libxkbcommon-x11 dependency, which was fixed before the passing test. This is not acceptance of every distribution, a Wayland session, clipboard, file-dialog provider or every workflow. Those remain on the platform acceptance matrix in `docs/linux-package.md`.

## 3. Clear branded installer

Generated artwork reuses ITGLA's existing logo. Welcome/header artwork uses deep green and paper colors, Segoe UI, short product-specific text and ?IT in a glance.? Four-times-resolution source bitmaps use FitControl. Welcome and directory pages were visually inspected at the current Windows DPI; no clipping was observed. Both the application and installer now expose 0.6.1 version, ITGLA company and product descriptions. Other display-DPI settings were not separately accepted.

## 4. Compression, integrity and signing

Solid LZMA reduces the complete Windows candidate setup from the published 11.54 MiB to approximately 8.44 MiB (26.9% smaller). This is an end-to-end candidate comparison; the candidate also includes current application source changes. The earlier same-payload experiment separately measured 29.5% compression savings.

The Windows build accepts an optional executable signing adapter: sign/verify the stripped main executable before ZIP/NSIS packaging, then sign/verify the final installer before checksums. `scripts/sign-windows.ps1` supports SHA-256 signing with an existing code-signing certificate, a trusted timestamp and strict verification. Its verify-only gate rejects the unsigned candidate as expected. No code-signing certificate was found in the current-user or local-machine personal stores; actual Windows signing and Apple notarization are not complete. No certificate was purchased, identity submitted or self-signed certificate substituted. See `docs/package-signing.md` for credential requirements and ordering.

Cross-platform release verification now requires all seven exact artifacts for a full-release audit, including both DMGs. Explicit Windows/Linux subset manifests support local testing. Checks cover hashes, portable ZIP CRC and contents, unsafe paths, embedded x64 GUI PE structure, data/debug-file leakage, installer/archive headers and DMG trailer. Native Linux archive/layout and ABI checks remain in `scripts/audit-packages.sh`. An incomplete set no longer passes by skipping missing packages. macOS packaging now writes into a clean temporary directory, preventing selection of a stale DMG from shared dist. Its syntax was checked, but neither DMG was built or notarized here.

## Validation evidence

- `output/installer-optimization-20261101/lifecycle-results.json`: nine actual NSIS lifecycle checks using compiled fixtures with independent default paths, registry and shortcuts. Covers fresh silent install/path with spaces, metadata, optional shortcut default, upgrade reuse, reinstall, downgrade rejection, occupied executable install/uninstall, replacement/recovery failure, unresolved recovery block and final data/unknown-file retention. The real product executable was separately launched with an isolated temporary workspace.
- `package-tests.log`: five release-verifier tests, including a five-package set rejected because both DMGs are missing, tampered download, unsafe ZIP and invalid DMG.
- `linux-smoke/`: actual clean-host DEB metadata, image ID, X11 window list, application log and screenshot.
- `linux-native-audit.json`, `appimage-final-abi.json`, `abi-rejection.json`: native layout/dependency and positive/negative ABI evidence.
- `windows-metadata.json`, `windows-launch.json`, `unsigned-rejection.log`: executable metadata, real candidate startup and expected signing rejection.
- Windows/Linux package-verification JSON, build-manifest and subset checksum manifests: final artifact hashes.
- Python/PowerShell parse checks, Bash syntax checks, NSIS compilation and scoped whitespace checks passed. No claim is made of complete application regression or macOS platform acceptance from these packaging checks.

## Test incident and recovery

The first lifecycle runner quoted the entire NSIS /D argument, so NSIS ignored it and used its default installation path. This temporarily placed the fixture program in the existing application folder. The original 0.3.27 installer was immediately used to restore known program files; the restored executable hash was verified against the original release ZIP, including after later testing. The production uninstall key remains at 0.3.27. No real workspace was opened or migrated. The runner now constructs NSIS's required unquoted final directory remainder, and fixture builds independently default to a test-only directory. Test fixture manifests additionally bind the expected independent registry key/default directory and exact executable hashes. Subsequent lifecycle runs passed in temporary directories and cleaned their custom registration/shortcuts. Unknown sentinel files were intentionally retained to prove uninstall safety.

## Remaining release gates

Provision real signing identities; run supported Windows lifecycle acceptance with real application migrations on copied databases; validate additional Linux distributions/desktop sessions; rebuild and accept both macOS DMGs in the next authorized release. Assign a new release version rather than overwriting the published 0.6.1 files. Windows/Linux continue to build locally or on 192.168.1.7; only macOS uses GitHub builds.
