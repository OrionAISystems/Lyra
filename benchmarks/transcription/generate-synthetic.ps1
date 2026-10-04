# Run with Windows PowerShell 5.1: System.Speech uses the installed Windows voice.
param([string]$Voice = 'Microsoft David Desktop', [string[]]$CaseId = @())
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Speech
$suite = Get-Content -LiteralPath (Join-Path $PSScriptRoot 'suite.json') -Raw | ConvertFrom-Json
foreach ($id in $CaseId) {
    if ($id -notin $suite.cases.id) { throw "Unknown case: $id" }
}
$audioRoot = Join-Path $PSScriptRoot 'audio'
[void](New-Item -ItemType Directory -Path $audioRoot -Force)
$selectedCases = @($suite.cases | Where-Object { $CaseId.Count -eq 0 -or $_.id -in $CaseId })
# Fail before generating anything when a selected fixture already exists.
foreach ($case in $selectedCases) {
    $path = Join-Path $PSScriptRoot $case.audio
    if (Test-Path -LiteralPath $path) { throw "Fixture exists: $path. Use a new corpus directory or deliberately remove generated fixtures first." }
}
$provenancePath = Join-Path $audioRoot 'provenance.json'
$generations = @()
if (Test-Path -LiteralPath $provenancePath) {
    $prior = Get-Content -LiteralPath $provenancePath -Raw | ConvertFrom-Json
    if ($prior.schema -eq 2) {
        if (-not $prior.generations) { throw 'Invalid generation provenance: no generations.' }
        $generations = @($prior.generations)
    } else {
        # Retain legacy metadata verbatim, including any absent case IDs; do not
        # invent provenance for audio that predates this generation history.
        $generations = @($prior)
    }
}
$synth = New-Object System.Speech.Synthesis.SpeechSynthesizer
$synth.SelectVoice($Voice)
$format = New-Object System.Speech.AudioFormat.SpeechAudioFormatInfo -ArgumentList 16000, ([System.Speech.AudioFormat.AudioBitsPerSample]::Sixteen), ([System.Speech.AudioFormat.AudioChannel]::Mono)
try {
    $generation = [ordered]@{ provenance='synthetic'; generator='Windows System.Speech'; voice=$Voice; rate=$synth.Rate; generatedUtc=[DateTime]::UtcNow.ToString('o'); generatedCaseIds=@(); interruptedPauseMs=1500 }
    $generations += $generation
    foreach ($case in $selectedCases) {
        $path = Join-Path $PSScriptRoot $case.audio
        if (Test-Path -LiteralPath $path) { throw "Fixture exists: $path. Use a new corpus directory or deliberately remove generated fixtures first." }
        if ($case.id -eq 'silence') {
            $writer = New-Object System.IO.BinaryWriter([System.IO.File]::Create($path))
            try {
                $writer.Write([System.Text.Encoding]::ASCII.GetBytes('RIFF'))
                $writer.Write([int]64036)
                $writer.Write([System.Text.Encoding]::ASCII.GetBytes('WAVEfmt '))
                $writer.Write([int]16); $writer.Write([int16]1); $writer.Write([int16]1)
                $writer.Write([int]16000); $writer.Write([int]32000)
                $writer.Write([int16]2); $writer.Write([int16]16)
                $writer.Write([System.Text.Encoding]::ASCII.GetBytes('data'))
                $writer.Write([int]64000); $writer.Write((New-Object byte[] 64000))
            } finally { $writer.Dispose() }
        } else {
            $synth.SetOutputToWaveFile($path, $format)
            $prompt = New-Object System.Speech.Synthesis.PromptBuilder
            switch ($case.id) {
                'punctuation' { $prompt.AppendText('Hello comma Orion period new line Please send the report period') }
                'numbers' { $prompt.AppendText('The order contains twelve boxes and costs forty five dollars.') }
                'decimal' { $prompt.AppendText('The temperature is minus twelve point five degrees.') }
                'currency' { $prompt.AppendText('The total is forty five dollars and fifty cents.') }
                'percentage' { $prompt.AppendText('The discount is twelve point five percent.') }
                'standalone-numbers' { $prompt.AppendText('We counted twenty five and then one hundred and five.') }
                'self-correction' { $prompt.AppendText('Send the report on Thursday scratch that Send the report on Friday period') }
                'interrupted' {
                    $prompt.AppendText('We should finish the review')
                    $prompt.AppendBreak([TimeSpan]::FromMilliseconds(1500))
                    $prompt.AppendText('tomorrow after the team meeting.')
                }
                default { $prompt.AppendText($case.expected) }
            }
            $synth.Speak($prompt)
            $synth.SetOutputToNull()
        }
        # Save each completed case so a later synthesis failure cannot erase
        # evidence for retained files. Existing generation records are append-only.
        $generation.generatedCaseIds += $case.id
        [ordered]@{ schema=2; generations=$generations } | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $provenancePath -Encoding UTF8
    }
} finally { $synth.Dispose() }
Write-Host "Generated synthetic PCM16 mono 16 kHz fixtures using $Voice. These are not microphone recordings."
