param(
    [string]$NdkHome = $env:ANDROID_NDK_HOME,
    [ValidateRange(21, 100)]
    [int]$ApiLevel = 29,
    [ValidateRange(0, 1000)]
    [int]$Warmups = 2,
    [ValidateRange(1, 1000)]
    [int]$Samples = 20,
    [string]$Serial,
    [switch]$Reverse
)

$ErrorActionPreference = "Stop"

if ([string]::IsNullOrWhiteSpace($NdkHome)) {
    throw "Pass -NdkHome or set ANDROID_NDK_HOME."
}

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "../..")).Path
$linker = Join-Path $NdkHome "toolchains/llvm/prebuilt/windows-x86_64/bin/aarch64-linux-android$ApiLevel-clang.cmd"
if (-not (Test-Path $linker)) {
    throw "Android linker not found: $linker"
}

$adbPrefix = @()
if ($Serial) {
    $adbPrefix = @("-s", $Serial)
}

function Invoke-Adb {
    param([Parameter(ValueFromRemainingArguments = $true)][string[]]$Arguments)
    & adb @adbPrefix @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "adb failed with exit code $LASTEXITCODE"
    }
}

rustup target add aarch64-linux-android
if ($LASTEXITCODE -ne 0) {
    throw "rustup target add failed with exit code $LASTEXITCODE"
}

$env:CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER = $linker
cargo build --manifest-path (Join-Path $repoRoot "Cargo.toml") --release --target aarch64-linux-android -p android-core-runner
if ($LASTEXITCODE -ne 0) {
    throw "cargo build failed with exit code $LASTEXITCODE"
}

$binary = Join-Path $repoRoot "target/aarch64-linux-android/release/android-core-runner"
$remote = "/data/local/tmp/android-core-runner"

Invoke-Adb push $binary $remote | Out-Null
Invoke-Adb shell chmod 755 $remote | Out-Null

$model = (& adb @adbPrefix shell getprop ro.product.model).Trim()
$android = (& adb @adbPrefix shell getprop ro.build.version.release).Trim()
$sdk = (& adb @adbPrefix shell getprop ro.build.version.sdk).Trim()
$abi = (& adb @adbPrefix shell getprop ro.product.cpu.abi).Trim()
$board = (& adb @adbPrefix shell getprop ro.board.platform).Trim()

Write-Output "# device=$model"
Write-Output "# android=$android"
Write-Output "# api=$sdk"
Write-Output "# abi=$abi"
Write-Output "# board=$board"

$runnerArguments = @($remote, "--warmups", "$Warmups", "--samples", "$Samples")
if ($Reverse) {
    $runnerArguments += "--reverse"
}

try {
    Invoke-Adb shell @runnerArguments
} finally {
    & adb @adbPrefix shell rm -f $remote | Out-Null
}
