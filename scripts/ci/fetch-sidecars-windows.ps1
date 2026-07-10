# Baixa os binários oficiais de Pandoc e Typst (releases mais recentes do
# GitHub) e os coloca em src-tauri/binaries/ com o nome de sidecar que o
# Tauri espera (<nome>-<target-triple>.exe). Roda em runners Windows (x86_64).
$ErrorActionPreference = "Stop"

$RootDir = Resolve-Path "$PSScriptRoot/../.."
$BinDir = Join-Path $RootDir "src-tauri/binaries"
New-Item -ItemType Directory -Force -Path $BinDir | Out-Null

$Target = (rustc -vV | Select-String "^host:") -replace "host:\s*", ""
$Target = $Target.Trim()
Write-Host "==> Target triple: $Target"

$Work = New-Item -ItemType Directory -Force -Path (Join-Path $env:TEMP "markforge-sidecars-$(Get-Random)")
Push-Location $Work

Write-Host "==> Resolvendo última release do Pandoc"
$pandocRelease = Invoke-RestMethod -Uri "https://api.github.com/repos/jgm/pandoc/releases/latest"
$pandocAsset = $pandocRelease.assets | Where-Object { $_.name -match "windows-x86_64\.zip$" } | Select-Object -First 1
if (-not $pandocAsset) { throw "Não encontrei o asset windows-x86_64.zip do Pandoc" }
Write-Host "    $($pandocAsset.browser_download_url)"
Invoke-WebRequest -Uri $pandocAsset.browser_download_url -OutFile "pandoc.zip"
Expand-Archive -Path "pandoc.zip" -DestinationPath "pandoc_extracted"
$pandocExe = Get-ChildItem -Path "pandoc_extracted" -Recurse -Filter "pandoc.exe" | Select-Object -First 1
Copy-Item $pandocExe.FullName (Join-Path $BinDir "pandoc-$Target.exe")
Write-Host "==> Pandoc instalado em $BinDir\pandoc-$Target.exe"

Write-Host "==> Resolvendo última release do Typst"
$typstRelease = Invoke-RestMethod -Uri "https://api.github.com/repos/typst/typst/releases/latest"
$typstAsset = $typstRelease.assets | Where-Object { $_.name -match "x86_64-pc-windows-msvc\.zip$" } | Select-Object -First 1
if (-not $typstAsset) { throw "Não encontrei o asset x86_64-pc-windows-msvc.zip do Typst" }
Write-Host "    $($typstAsset.browser_download_url)"
Invoke-WebRequest -Uri $typstAsset.browser_download_url -OutFile "typst.zip"
Expand-Archive -Path "typst.zip" -DestinationPath "typst_extracted"
$typstExe = Get-ChildItem -Path "typst_extracted" -Recurse -Filter "typst.exe" | Select-Object -First 1
Copy-Item $typstExe.FullName (Join-Path $BinDir "typst-$Target.exe")
Write-Host "==> Typst instalado em $BinDir\typst-$Target.exe"

Pop-Location
Remove-Item -Recurse -Force $Work
