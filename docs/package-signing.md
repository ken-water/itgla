# Package signing and release order

Signing is a separate release gate. Development candidates may be unsigned but must be labeled unsigned. Branding, hdiutil verification and SHA-256 do not establish publisher trust. Never create a self-signed certificate and claim it removes SmartScreen or Gatekeeper warnings.

## Windows

Obtain an identity-validated code-signing certificate or managed signing service. Prefer hardware-backed/private-key protection. Do not store private keys or passwords in the repository or build logs.

`scripts/sign-windows.ps1 -Path <file> -CertificateThumbprint <thumbprint>` signs with SHA-256 and a trusted timestamp, then verifies both. `-VerifyOnly` rejects unsigned/untrusted/untimestamped files. The certificate must already exist in the current user's certificate store. No certificate purchase or identity submission is automated.

For Linux cross-packaging, `ITGLA_WINDOWS_SIGNER` may point to an executable adapter that accepts exactly one file path, signs and verifies it, and returns nonzero on failure. The packaging script invokes it after stripping the program, before ZIP/NSIS compression, then on the finished installer before checksums. It is an executable path, not a shell expression. A remote Windows signer may wrap the PowerShell script; secure transport and key custody are deployment responsibilities. The default build is unsigned until a signer is configured.

Order: build/strip executable; sign and verify executable; create ZIP and NSIS; sign and verify installer; generate checksums and manifest. Signing an executable after ZIP/installer creation leaves unsigned embedded copies. Any post-signing mutation invalidates the signature. Do not change binary resources or strip after signing.

## macOS

A release needs an Apple Developer identity, hardened-runtime signing of the app and nested executable code, Apple notarization and a stapled ticket. Check the mounted app with `codesign --verify --deep --strict --verbose=2`, `spctl --assess --type execute --verbose=2`, and validate the stapled ticket. Verify both Apple silicon and Intel packages on supported macOS hosts. `hdiutil verify` alone checks the image, not Gatekeeper acceptance. Keep credentials in the approved CI secret store and never print them.

The current macOS packaging script remains an unsigned development path; no signing/notarization success is claimed. Configure the real identity and notarization only when credentials are supplied. Recompute checksums after all signing/stapling and packaging steps. Build only the two DMGs on GitHub; Windows and Linux stay local/192.168.1.7.
