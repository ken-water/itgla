param(
    [Parameter(Mandatory=$true)][string[]]$Path,
    [string]$CertificateThumbprint,
    [switch]$VerifyOnly,
    [string]$TimestampServer = 'http://timestamp.digicert.com'
)
$ErrorActionPreference = 'Stop'
$resolved = @($Path | ForEach-Object { (Resolve-Path -LiteralPath $_).Path })
if (-not $VerifyOnly) {
    if (-not $CertificateThumbprint) { throw 'Supply a real code-signing certificate thumbprint; unsigned builds cannot pass signing.' }
    $certificate = Get-Item -LiteralPath "Cert:\CurrentUser\My\$CertificateThumbprint"
    if (-not $certificate.HasPrivateKey -or $certificate.NotAfter -lt (Get-Date)) { throw 'Certificate is expired or has no private key.' }
    if (-not ($certificate.EnhancedKeyUsageList | Where-Object ObjectId -eq '1.3.6.1.5.5.7.3.3')) { throw 'Certificate does not allow code signing.' }
    foreach ($file in $resolved) {
        $result = Set-AuthenticodeSignature -LiteralPath $file -Certificate $certificate -HashAlgorithm SHA256 -TimestampServer $TimestampServer
        if ($result.Status -ne 'Valid' -or -not $result.TimeStamperCertificate) { throw "Signing or trusted timestamp failed: $file" }
    }
}
foreach ($file in $resolved) {
    $signature = Get-AuthenticodeSignature -LiteralPath $file
    if ($signature.Status -ne 'Valid' -or -not $signature.TimeStamperCertificate) { throw "Signature/timestamp verification failed: $file ($($signature.Status))" }
    [pscustomobject]@{File=$file;Status=$signature.Status;Publisher=$signature.SignerCertificate.Subject;SHA256=(Get-FileHash -LiteralPath $file -Algorithm SHA256).Hash}
}
