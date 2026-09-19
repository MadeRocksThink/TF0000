param(
    [Parameter(Mandatory = $true)]
    [ValidatePattern('^[a-p]{32}$')]
    [string]$ExtensionId,

    [ValidateSet('Chrome', 'Edge', 'Both')]
    [string]$Browser = 'Both'
)

$ErrorActionPreference = 'Stop'
$repositoryRoot = Split-Path -Parent $PSScriptRoot
$hostExecutable = Join-Path $repositoryRoot 'target\release\tf0000-native-host.exe'
if (-not (Test-Path -LiteralPath $hostExecutable -PathType Leaf)) {
    throw "Native host is not built. Run: cargo build --release -p tf0000-native-host"
}

$manifestDirectory = Join-Path $env:LOCALAPPDATA 'TF0000\NativeMessaging'
New-Item -ItemType Directory -Path $manifestDirectory -Force | Out-Null
$manifestPath = Join-Path $manifestDirectory 'com.tf0000.context.json'
$manifest = [ordered]@{
    name = 'com.tf0000.context'
    description = 'TF0000 local storage bridge'
    path = $hostExecutable
    type = 'stdio'
    allowed_origins = @("chrome-extension://$ExtensionId/")
}
$manifest | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath $manifestPath -Encoding utf8

$registryTargets = switch ($Browser) {
    'Chrome' { 'HKCU:\Software\Google\Chrome\NativeMessagingHosts\com.tf0000.context' }
    'Edge' { 'HKCU:\Software\Microsoft\Edge\NativeMessagingHosts\com.tf0000.context' }
    'Both' {
        'HKCU:\Software\Google\Chrome\NativeMessagingHosts\com.tf0000.context'
        'HKCU:\Software\Microsoft\Edge\NativeMessagingHosts\com.tf0000.context'
    }
}
foreach ($registryPath in $registryTargets) {
    New-Item -Path $registryPath -Force | Out-Null
    Set-Item -Path $registryPath -Value $manifestPath
}

Write-Host "Registered com.tf0000.context for $Browser."
Write-Host "Manifest: $manifestPath"
