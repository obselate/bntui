$toolsDir = "$(Split-Path -Parent $MyInvocation.MyCommand.Definition)"
$exePath = Join-Path $toolsDir 'bntui.exe'

if (Test-Path $exePath) {
  Remove-Item $exePath -Force
}
