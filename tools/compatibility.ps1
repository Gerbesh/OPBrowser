param(
    [string]$Test262Path = "",
    [int]$Test262Limit = 2000
)

$ErrorActionPreference = "Stop"
$Cargo = Join-Path $env:USERPROFILE ".cargo\bin\cargo.exe"
if (-not (Test-Path $Cargo)) {
    $Cargo = (Get-Command cargo -ErrorAction Stop).Source
}

function Invoke-CargoCheck {
    param([string]$Name, [string[]]$Arguments)
    Write-Host ""
    Write-Host "== $Name =="
    & $Cargo @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "$Name failed with exit code $LASTEXITCODE"
    }
}

Write-Host "OPBrowser compatibility probe"
Write-Host "Subsystem baseline only; not a browser-wide conformance claim."

Invoke-CargoCheck "HTML project tests" @("test", "-p", "op_html", "--quiet")
Invoke-CargoCheck "CSS project tests" @("test", "-p", "op_css", "--quiet")
Invoke-CargoCheck "Layout project tests" @("test", "-p", "op_layout", "--quiet")
Invoke-CargoCheck "Engine integration tests" @("test", "-p", "op_engine", "--quiet")
Invoke-CargoCheck "JavaScript project tests" @("test", "-p", "op_js", "--quiet")

if ($Test262Path) {
    $Resolved = (Resolve-Path $Test262Path).Path
    Write-Host ""
    Write-Host "== Test262 parse probe =="
    & $Cargo run -p op_js --quiet --bin test262_probe -- $Resolved --limit $Test262Limit
    if ($LASTEXITCODE -ne 0) {
        throw "Test262 parse probe failed with exit code $LASTEXITCODE"
    }
} else {
    Write-Host ""
    Write-Host "Test262 parse probe skipped. Pass -Test262Path <path-to-test262-test> to measure it."
}

Write-Host ""
Write-Host "WPT browser automation is not wired yet. Project-owned test counts are not a WPT percentage."
