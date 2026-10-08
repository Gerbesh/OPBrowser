param(
    [string]$Test262Path = "",
    [string]$WptPath = "",
    [string]$OutputDir = "artifacts/compatibility",
    [switch]$ExternalOnly,
    [switch]$RuntimeOnly
)

$ErrorActionPreference = "Stop"
$RepoRoot = Split-Path $PSScriptRoot -Parent
$Cargo = Join-Path $env:USERPROFILE ".cargo\bin\cargo.exe"
if (-not (Test-Path $Cargo)) {
    $Cargo = (Get-Command cargo -ErrorAction Stop).Source
}

if ([System.IO.Path]::IsPathRooted($OutputDir)) {
    $ResolvedOutput = $OutputDir
} else {
    $ResolvedOutput = Join-Path $RepoRoot $OutputDir
}
New-Item -ItemType Directory -Path $ResolvedOutput -Force | Out-Null

function Invoke-CargoCheck {
    param([string]$Name, [string[]]$Arguments)
    Write-Host ""
    Write-Host "== $Name =="
    & $Cargo @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "$Name failed with exit code $LASTEXITCODE"
    }
}

function Write-ShieldsBadge {
    param(
        [string]$Path,
        [string]$Label,
        [double]$Percent,
        [int]$Passed,
        [int]$Total
    )
    $Badge = @{
        schemaVersion = 1
        label = $Label
        message = ("{0:N2}% ({1}/{2})" -f $Percent, $Passed, $Total)
        color = "informational"
    } | ConvertTo-Json -Compress
    Set-Content -Path $Path -Value $Badge -Encoding utf8
}

Write-Host "OPBrowser compatibility measurement"
Write-Host "Project-owned regressions and external conformance subsets are reported separately."

Push-Location $RepoRoot
try {
    if (-not $ExternalOnly) {
        Invoke-CargoCheck "HTML project tests" @("test", "-p", "op_html", "--quiet")
        Invoke-CargoCheck "CSS project tests" @("test", "-p", "op_css", "--quiet")
        Invoke-CargoCheck "Layout project tests" @("test", "-p", "op_layout", "--quiet")
        Invoke-CargoCheck "Engine integration tests" @("test", "-p", "op_engine", "--quiet")
        Invoke-CargoCheck "JavaScript project tests" @("test", "-p", "op_js", "--quiet")
    }

    $Summary = @("# OPBrowser compatibility", "")

    if ($Test262Path) {
        $ResolvedTest262 = (Resolve-Path $Test262Path).Path
        if (-not $RuntimeOnly) {
            $Test262Manifest = Join-Path $RepoRoot "compat\test262-parser-v1.txt"
            $Test262Json = Join-Path $ResolvedOutput "test262-parser-v1.json"
            Write-Host ""
            Write-Host "== Test262 parser subset v1 =="
            & $Cargo run -p op_js --quiet --bin test262_probe -- $ResolvedTest262 --manifest $Test262Manifest --json-out $Test262Json
            if ($LASTEXITCODE -ne 0) {
                throw "Test262 parser subset failed with exit code $LASTEXITCODE"
            }
            $Metric = Get-Content $Test262Json -Raw | ConvertFrom-Json
            Write-ShieldsBadge -Path (Join-Path $ResolvedOutput "test262-parser-v1-badge.json") -Label "Test262 parser v1" -Percent $Metric.percent -Passed $Metric.passed -Total $Metric.total
            $Summary += ("- Test262 parser v1: **{0:N2}%** ({1}/{2}), upstream {3}" -f $Metric.percent, $Metric.passed, $Metric.total, $Metric.upstream)
        } else {
            $Summary += "- Test262 parser v1: not run (RuntimeOnly)"
        }

        $RuntimeManifest = Join-Path $RepoRoot "compat\test262-runtime-v1.txt"
        $RuntimeJson = Join-Path $ResolvedOutput "test262-runtime-v1.json"
        Write-Host ""
        Write-Host "== Test262 runtime classic-script subset v1 =="
        & $Cargo run -p op_js --quiet --bin test262_runtime_probe -- $ResolvedTest262 --manifest $RuntimeManifest --json-out $RuntimeJson
        if ($LASTEXITCODE -ne 0) {
            throw "Test262 runtime probe failed with exit code $LASTEXITCODE"
        }
        $RuntimeMetric = Get-Content $RuntimeJson -Raw | ConvertFrom-Json
        Write-ShieldsBadge -Path (Join-Path $ResolvedOutput "test262-runtime-v1-badge.json") -Label "Test262 runtime v1" -Percent $RuntimeMetric.percent -Passed $RuntimeMetric.passed -Total $RuntimeMetric.attempted
        $Summary += ("- Test262 runtime v1: **{0:N2}%** ({1}/{2} attempted); {3} explicitly skipped; pinned revision verified={4}. Narrow classic-script sample only." -f $RuntimeMetric.percent, $RuntimeMetric.passed, $RuntimeMetric.attempted, $RuntimeMetric.skipped, $RuntimeMetric.revision_verified)
    } else {
        Write-Host ""
        Write-Host "Test262 parser and runtime subsets skipped. Pass -Test262Path <path-to-test262-test>."
        $Summary += "- Test262 parser v1: not run"
        $Summary += "- Test262 runtime v1: not run"
    }

    if ($WptPath) {
        $ResolvedWpt = (Resolve-Path $WptPath).Path
        $WptManifest = Join-Path $RepoRoot "compat\wpt-static-v1.tsv"
        $WptJson = Join-Path $ResolvedOutput "wpt-static-v1.json"
        Write-Host ""
        Write-Host "== WPT static reftest subset v1 =="
        & $Cargo run -p op_browser --quiet --bin wpt_probe -- $ResolvedWpt $WptManifest --json-out $WptJson
        if ($LASTEXITCODE -ne 0) {
            throw "WPT static subset failed with exit code $LASTEXITCODE"
        }
        $Metric = Get-Content $WptJson -Raw | ConvertFrom-Json
        Write-ShieldsBadge -Path (Join-Path $ResolvedOutput "wpt-static-v1-badge.json") -Label "WPT static v1" -Percent $Metric.percent -Passed $Metric.passed -Total $Metric.total
        $Summary += ("- WPT static v1: **{0:N2}%** ({1}/{2}), upstream {3}" -f $Metric.percent, $Metric.passed, $Metric.total, $Metric.upstream)

        $WptPositioningManifest = Join-Path $RepoRoot "compat\wpt-positioning-v1.tsv"
        $WptPositioningJson = Join-Path $ResolvedOutput "wpt-positioning-v1.json"
        Write-Host ""
        Write-Host "== WPT positioning/visual-formatting subset v1 =="
        & $Cargo run -p op_browser --quiet --bin wpt_probe -- $ResolvedWpt $WptPositioningManifest --json-out $WptPositioningJson
        if ($LASTEXITCODE -ne 0) {
            throw "WPT positioning subset failed with exit code $LASTEXITCODE"
        }
        $PositioningMetric = Get-Content $WptPositioningJson -Raw | ConvertFrom-Json
        Write-ShieldsBadge -Path (Join-Path $ResolvedOutput "wpt-positioning-v1-badge.json") -Label "WPT positioning v1" -Percent $PositioningMetric.percent -Passed $PositioningMetric.passed -Total $PositioningMetric.total
        $Summary += ("- WPT positioning v1: **{0:N2}%** ({1}/{2}), upstream {3}" -f $PositioningMetric.percent, $PositioningMetric.passed, $PositioningMetric.total, $PositioningMetric.upstream)
    } else {
        Write-Host ""
        Write-Host "WPT static subset skipped. Pass -WptPath <path-to-wpt-checkout>."
        $Summary += "- WPT static v1: not run"
    }

    $Summary += ""
    $Summary += "These percentages describe only the named, versioned subsets. They are not full WPT or full ECMAScript conformance scores."
    Set-Content -Path (Join-Path $ResolvedOutput "summary.md") -Value $Summary -Encoding utf8

    Write-Host ""
    Write-Host "Compatibility artifacts: $ResolvedOutput"
} finally {
    Pop-Location
}
