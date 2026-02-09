$ErrorActionPreference = 'Stop'

$packageName = 'bntui'
$version = '0.1.2'
$url64 = "https://github.com/obselate/bntui/releases/download/v${version}/bntui-windows-x86_64.exe"

$toolsDir = "$(Split-Path -Parent $MyInvocation.MyCommand.Definition)"
$exePath = Join-Path $toolsDir 'bntui.exe'

Get-ChocolateyWebFile -PackageName $packageName `
  -FileFullPath $exePath `
  -Url64bit $url64 `
  -Checksum64 'REPLACE_WITH_SHA256_AFTER_RELEASE_BUILD' `
  -ChecksumType64 'sha256'
