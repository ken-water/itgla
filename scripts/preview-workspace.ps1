param(
    [ValidateSet('overview', 'assets', 'reminders', 'search', 'details', 'connections', 'settings', 'guide', 'backup', 'recovery')]
    [string]$View = 'overview',
    [switch]$LargeDark,
    [ValidatePattern('^\d+x\d+$')]
    [string]$WindowSize = '1440x900'
)

$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot
$executable = Join-Path $repoRoot 'target/debug/itgla.exe'
if (-not (Test-Path -LiteralPath $executable)) {
    throw 'Build the native debug candidate before opening this preview.'
}

# Always isolate the preview: opening the candidate must not migrate a real workspace.
$previewDirectory = Join-Path ([IO.Path]::GetTempPath()) ('itgla-workspace-preview-' + [guid]::NewGuid().ToString('N'))
$names = @('ITGLA_DATA_DIR', 'ITGLA_AUDIT_DEMO', 'ITGLA_AUDIT_VIEW', 'ITGLA_AUDIT_LARGE_DARK', 'ITGLA_AUDIT_WINDOW_SIZE', 'ITGLA_AUDIT_SNAPSHOT')
$previous = @{}
foreach ($name in $names) { $previous[$name] = [Environment]::GetEnvironmentVariable($name, 'Process') }
try {
    $env:ITGLA_DATA_DIR = $previewDirectory
    $env:ITGLA_AUDIT_DEMO = '1'
    $env:ITGLA_AUDIT_VIEW = $View
    $env:ITGLA_AUDIT_WINDOW_SIZE = $WindowSize
    [Environment]::SetEnvironmentVariable('ITGLA_AUDIT_LARGE_DARK', $(if ($LargeDark) { '1' } else { $null }), 'Process')
    [Environment]::SetEnvironmentVariable('ITGLA_AUDIT_SNAPSHOT', $null, 'Process')
    $app = Start-Process -FilePath $executable -WorkingDirectory $repoRoot -PassThru
    [pscustomobject]@{ ProcessId = $app.Id; Executable = $executable; IsolatedWorkspace = $previewDirectory }
} finally {
    foreach ($name in $names) { [Environment]::SetEnvironmentVariable($name, $previous[$name], 'Process') }
}
