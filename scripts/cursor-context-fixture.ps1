param(
    [switch]$Fixture,
    [ValidateSet('empty', 'continuation', 'middle', 'bounded', 'boundary', 'selection', 'password', 'unsupported')]
    [string]$Case = 'continuation',
    [string]$MetadataPath,
    [string]$TestBinary,
    [ValidateSet('empty', 'continuation', 'middle', 'bounded', 'boundary', 'selection', 'password', 'unsupported')]
    [string[]]$Cases = @('empty', 'continuation', 'middle', 'bounded', 'boundary', 'selection', 'password', 'unsupported')
)

$ErrorActionPreference = 'Stop'

if ($Fixture) {
    Add-Type -AssemblyName System.Windows.Forms
    Add-Type -AssemblyName System.Drawing
    Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class LyraCursorFixtureNative {
    [StructLayout(LayoutKind.Sequential)] public struct Point { public int X; public int Y; }
    [DllImport("user32.dll")] public static extern IntPtr WindowFromPoint(Point point);
    [DllImport("user32.dll")] public static extern IntPtr GetAncestor(IntPtr hwnd, uint flags);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hwnd, int command);
    [DllImport("user32.dll")] public static extern IntPtr SetThreadDpiAwarenessContext(IntPtr context);
    [DllImport("user32.dll")] public static extern void mouse_event(uint flags, uint x, uint y, uint data, UIntPtr extra);
}
'@
    # Keep native hit testing and WinForms screen points in the same coordinates on scaled monitors.
    [LyraCursorFixtureNative]::SetThreadDpiAwarenessContext([IntPtr]::new(-4)) | Out-Null
    $form = New-Object System.Windows.Forms.Form
    $form.Text = 'Lyra synthetic cursor context fixture'
    $form.Size = New-Object System.Drawing.Size(520, 220)
    $form.StartPosition = 'CenterScreen'
    $form.TopMost = $true
    $field = if ($Case -in @('password', 'unsupported')) {
        $passwordField = New-Object System.Windows.Forms.TextBox
        $passwordField.UseSystemPasswordChar = $Case -eq 'password'
        $passwordField
    } else { New-Object System.Windows.Forms.RichTextBox }
    $field.Dock = 'Fill'
    $field.Text = switch ($Case) {
        'empty' { '' }
        'boundary' { 'finished. ' }
        'bounded' { ('Older sentence. ' * 30) + 'typed text ' }
        'middle' { 'typed text Trailing sentence ends here.' }
        default { 'typed text ' }
    }
    $form.Controls.Add($field)
    $timer = New-Object System.Windows.Forms.Timer
    $timer.Interval = 300
    $timer.Add_Tick({
        $timer.Stop()
        [LyraCursorFixtureNative]::ShowWindow($form.Handle, 5) | Out-Null
        $form.Activate()
        # Windows may deny programmatic foreground activation from this background runner.
        # Click only a point verified to belong to this synthetic form, then restore the pointer.
        $clickPoint = $field.PointToScreen((New-Object System.Drawing.Point(10, 10)))
        $nativePoint = New-Object LyraCursorFixtureNative+Point
        $nativePoint.X = $clickPoint.X
        $nativePoint.Y = $clickPoint.Y
        $hitRoot = [LyraCursorFixtureNative]::GetAncestor([LyraCursorFixtureNative]::WindowFromPoint($nativePoint), 2)
        @{ target = $form.Handle.ToInt64(); foreground = [LyraCursorFixtureNative]::GetForegroundWindow().ToInt64(); hitRoot = $hitRoot.ToInt64() } | ConvertTo-Json | Set-Content -LiteralPath ($MetadataPath + '.state')
        if ($hitRoot -eq $form.Handle) {
            $previousPointer = [System.Windows.Forms.Cursor]::Position
            try {
                [System.Windows.Forms.Cursor]::Position = $clickPoint
                [LyraCursorFixtureNative]::mouse_event(2, 0, 0, 0, [UIntPtr]::Zero)
                [LyraCursorFixtureNative]::mouse_event(4, 0, 0, 0, [UIntPtr]::Zero)
                Start-Sleep -Milliseconds 75
                [System.Windows.Forms.Application]::DoEvents()
            } finally { [System.Windows.Forms.Cursor]::Position = $previousPointer }
        }
        if ([LyraCursorFixtureNative]::GetForegroundWindow() -ne $form.Handle) {
            $timer.Start()
            return
        }
        if (-not $field.Focus()) { $timer.Start(); return }
        $field.SelectionStart = if ($Case -eq 'middle') { 'typed text '.Length } else { $field.Text.Length }
        $field.SelectionLength = 0
        if ($Case -eq 'selection') { $field.Select(0, 5) }
        @{ pid = $PID; hwnd = $form.Handle.ToInt64() } | ConvertTo-Json | Set-Content -LiteralPath $MetadataPath
    })
    $form.Add_Shown({ $timer.Start() })
    [System.Windows.Forms.Application]::Run($form)
    $timer.Dispose()
    $form.Dispose()
    exit
}

if (-not $TestBinary) {
    throw 'Pass the compiled regression-bench test executable via -TestBinary.'
}
$TestBinary = (Resolve-Path -LiteralPath $TestBinary).Path
$fixtureDirectory = Join-Path ([System.IO.Path]::GetTempPath()) ('lyra-cursor-fixture-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $fixtureDirectory | Out-Null
$savedEnvironment = @{}
foreach ($key in 'LYRA_CURSOR_FIXTURE_PID', 'LYRA_CURSOR_FIXTURE_HWND', 'LYRA_CURSOR_FIXTURE_CASE') {
    $savedEnvironment[$key] = [Environment]::GetEnvironmentVariable($key)
}
try {
    foreach ($fixtureCase in $Cases) {
        $metadata = Join-Path $fixtureDirectory ($fixtureCase + '.json')
        $fixtureProcess = Start-Process powershell.exe -WindowStyle Hidden -PassThru -ArgumentList @(
            '-NoProfile', '-STA', '-File', ('"' + $PSCommandPath + '"'), '-Fixture', '-Case', $fixtureCase,
            '-MetadataPath', ('"' + $metadata + '"')
        )
        try {
            $deadline = [DateTime]::UtcNow.AddSeconds(10)
            while (-not (Test-Path -LiteralPath $metadata)) {
                if ($fixtureProcess.HasExited -or [DateTime]::UtcNow -ge $deadline) {
                    if (Test-Path -LiteralPath ($metadata + '.state')) { Get-Content -LiteralPath ($metadata + '.state') | Write-Output }
                    throw "Synthetic fixture did not become ready: $fixtureCase"
                }
                Start-Sleep -Milliseconds 100
            }
            $identity = Get-Content -Raw -LiteralPath $metadata | ConvertFrom-Json
            if ($identity.pid -ne $fixtureProcess.Id) { throw 'Unexpected synthetic fixture process.' }
            $env:LYRA_CURSOR_FIXTURE_PID = [string]$identity.pid
            $env:LYRA_CURSOR_FIXTURE_HWND = [string]$identity.hwnd
            $env:LYRA_CURSOR_FIXTURE_CASE = $fixtureCase
            & $TestBinary 'cursor_context::tests::native_cursor_context_fixture' --exact --ignored --nocapture
            if ($LASTEXITCODE -ne 0) { throw "Cursor context fixture failed: $fixtureCase" }
        } finally {
            if (-not $fixtureProcess.HasExited) {
                $fixtureProcess.CloseMainWindow() | Out-Null
                if (-not $fixtureProcess.WaitForExit(2000)) { Stop-Process -Id $fixtureProcess.Id }
            }
        }
    }
} finally {
    foreach ($key in $savedEnvironment.Keys) { [Environment]::SetEnvironmentVariable($key, $savedEnvironment[$key]) }
    # The exact directory is created above beneath TEMP; remove only its metadata files.
    Get-ChildItem -LiteralPath $fixtureDirectory -File | Remove-Item
    Remove-Item -LiteralPath $fixtureDirectory
}
