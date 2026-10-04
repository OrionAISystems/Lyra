# Windows PowerShell 5.1 / installed System.Speech voice required. No app models.
param([string]$Voice = 'Microsoft David Desktop')
$ErrorActionPreference = 'Stop'
$testRoot = Join-Path ([System.IO.Path]::GetTempPath()) ('lyra-generator-test-' + [Guid]::NewGuid().ToString('N'))
[void](New-Item -ItemType Directory -Path $testRoot)
try {
    Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'generate-synthetic.ps1') -Destination $testRoot
    Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'suite.json') -Destination $testRoot
    $script = Join-Path $testRoot 'generate-synthetic.ps1'
    $sidecar = Join-Path $testRoot 'audio/provenance.json'
    & $script -Voice $Voice -CaseId @('short')
    $first = Get-Content -LiteralPath $sidecar -Raw | ConvertFrom-Json
    if ($first.schema -ne 2 -or $first.generations.Count -ne 1 -or $first.generations[0].generatedCaseIds[0] -ne 'short') { throw 'Initial provenance missing.' }
    $retainedHash = (Get-FileHash -LiteralPath (Join-Path $testRoot 'audio/short.wav')).Hash
    # Exercise extension of legacy corpus metadata as well as the new history.
    $legacy = $first.generations[0] | ConvertTo-Json -Depth 10 -Compress
    $legacy | Set-Content -LiteralPath $sidecar -Encoding UTF8
    & $script -Voice $Voice -CaseId @('number-prose')
    & $script -Voice $Voice -CaseId @('silence')
    $extended = Get-Content -LiteralPath $sidecar -Raw | ConvertFrom-Json
    if ($extended.schema -ne 2 -or $extended.generations.Count -ne 3) { throw 'Generation history was replaced.' }
    if (($extended.generations[0] | ConvertTo-Json -Depth 10 -Compress) -ne $legacy) { throw 'Retained legacy metadata changed.' }
    if ($extended.generations[1].generatedCaseIds[0] -ne 'number-prose' -or $extended.generations[2].generatedCaseIds[0] -ne 'silence') { throw 'New generation metadata missing.' }
    if ((Get-FileHash -LiteralPath (Join-Path $testRoot 'audio/short.wav')).Hash -ne $retainedHash) { throw 'Retained WAV changed.' }
    $sidecarHash = (Get-FileHash -LiteralPath $sidecar).Hash
    $refused = $false
    try { & $script -Voice $Voice -CaseId @('numbers', 'short') } catch {
        if ($_.Exception.Message -notlike 'Fixture exists:*') { throw }
        $refused = $true
    }
    if (-not $refused) { throw 'Existing WAV was not refused.' }
    if (Test-Path -LiteralPath (Join-Path $testRoot 'audio/numbers.wav')) { throw 'Preflight failure generated a new WAV.' }
    if ((Get-FileHash -LiteralPath $sidecar).Hash -ne $sidecarHash) { throw 'Preflight failure changed provenance.' }
    Write-Host 'Generator regression checks passed: initial history, legacy extension, repeated extension, retained WAV, duplicate preflight.'
} finally {
    $resolved = [System.IO.Path]::GetFullPath($testRoot)
    $tempPrefix = [System.IO.Path]::GetFullPath([System.IO.Path]::GetTempPath()).TrimEnd('\') + '\'
    if (-not $resolved.StartsWith($tempPrefix, [StringComparison]::OrdinalIgnoreCase) -or (Split-Path -Leaf $resolved) -notlike 'lyra-generator-test-*') { throw 'Refusing cleanup outside the generator test directory.' }
    Remove-Item -LiteralPath $resolved -Recurse -Force
}
