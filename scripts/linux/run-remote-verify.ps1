# Run AppImage UI verification on the Linux hardware test machine from Windows.
param(
  [string]$LinuxHost = $env:LINUX_TEST_HOST,
  [string]$LinuxUser = $env:LINUX_TEST_USER,
  [string]$ReleaseTag = $env:TANDEM_RELEASE_TAG
)

$ErrorActionPreference = "Stop"
$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "..\..")
$EnvFile = Join-Path $PSScriptRoot "remote-test.env"

if (Test-Path $EnvFile) {
  Get-Content $EnvFile | ForEach-Object {
    if ($_ -match '^\s*([A-Za-z_][A-Za-z0-9_]*)=(.*)$') {
      $name = $matches[1]
      $value = $matches[2]
      if (-not (Get-Item "Env:$name" -ErrorAction SilentlyContinue)) {
        Set-Item "Env:$name" $value
      }
    }
  }
}

if (-not $LinuxHost) { $LinuxHost = $env:LINUX_TEST_HOST }
if (-not $LinuxUser) { $LinuxUser = $env:LINUX_TEST_USER }
if (-not $ReleaseTag) { $ReleaseTag = $env:TANDEM_RELEASE_TAG }

if (-not $LinuxHost) { $LinuxHost = "192.168.3.210" }
if (-not $LinuxUser) { $LinuxUser = "finn-rm" }
if (-not $ReleaseTag) { $ReleaseTag = "v1.1.6" }

$Remote = "${LinuxUser}@${LinuxHost}"
$AppImageUrl = "https://github.com/nerif-tafu/tandem-desktop/releases/download/${ReleaseTag}/Tandem-linux-x86_64.AppImage"
$RemoteAppImage = "~/tandem-test/Tandem-linux-x86_64.AppImage"

Write-Host "Target: $Remote"
Write-Host "Release: $ReleaseTag"

ssh $Remote "mkdir -p ~/tandem-test"

$Scripts = @(
  "verify-appimage-ui.sh",
  "install-remote-test-deps.sh",
  "remote-run-verify.sh"
)

foreach ($script in $Scripts) {
  $local = Join-Path $PSScriptRoot $script
  scp $local "${Remote}:~/tandem-test/$script"
}

ssh $Remote "sed -i 's/\r$//' ~/tandem-test/*.sh; chmod +x ~/tandem-test/*.sh; bash ~/tandem-test/remote-run-verify.sh $ReleaseTag"
