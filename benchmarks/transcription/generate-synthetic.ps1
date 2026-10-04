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
$synth = New-Object System.Speech.Synthesis.SpeechSynthesizer
$synth.SelectVoice($Voice)
$format = New-Object System.Speech.AudioFormat.SpeechAudioFormatInfo -ArgumentList 16000, ([System.Speech.AudioFormat.AudioBitsPerSample]::Sixteen), ([System.Speech.AudioFormat.AudioChannel]::Mono)
try {
    foreach ($case in $suite.cases) {
        if ($CaseId.Count -gt 0 -and $case.id -notin $CaseId) { continue }
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
            continue
        }
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
    [ordered]@{ provenance='synthetic'; generator='Windows System.Speech'; voice=$Voice; rate=$synth.Rate; generatedUtc=[DateTime]::UtcNow.ToString('o'); generatedCaseIds=@($suite.cases | Where-Object { $CaseId.Count -eq 0 -or $_.id -in $CaseId } | ForEach-Object { $_.id }); interruptedPauseMs=1500 } | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $audioRoot 'provenance.json') -Encoding UTF8
} finally { $synth.Dispose() }
Write-Host "Generated synthetic PCM16 mono 16 kHz fixtures using $Voice. These are not microphone recordings."
