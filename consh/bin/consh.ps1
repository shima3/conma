# consh.ps1: start ConSh from PowerShell on Windows, as consh/bin/consh does from bash.
#   consh.ps1 [--conma-parser] [--dump] <script> [<arg> ...]
#   consh.ps1                 interactive mode (reads standard input line by line)
# It runs the ConMa version of ConSh on the Rust VM. The Rust version (consh/rust, consh-rs) does not build on
# Windows (it uses Unix-only APIs), so CONSH_IMPL unset or rust runs the ConMa version too, unless
# consh/rust/target/release/consh-rs.exe is there. CONSH_IMPL=reference runs the reference version.
# CONMA_VM picks the VM: rust (vm/target/release/conma.exe, the default) or transpiled
# (vm/target-transpiled/release/conma.exe). The Python VM and the wasm VM do not run here.
# --python-parser is not taken (consh/bin/consh does it with Python's consh_parse).
# CONSH_CONMA_DATA and CONSH_MODULE_DIR mean what they mean to consh/bin/consh.
# The module is kept in the directory that bin/cache-dir writes (the same checksum of the path of the repository,
# as Git Bash writes it), so this script and consh/bin/consh share it. It is made again with conma build when a .se
# file or a directory under the include directories, or conma.exe, is newer than it.
# If conma.exe is older than its sources (after git pull, for example), it warns once, with the path from the current
# directory, and runs it; in the interactive mode with a console and cargo, it first asks whether to build it now,
# as consh/bin/consh does (Update-StaleBuild).
# Commands such as ls, cat and grep are programs of Git Bash on Windows: when ls.exe is not on PATH and
# Git's usr\bin is there, it is added at the end of PATH while ConSh runs (and taken out again after).
# If PowerShell does not run scripts (the execution policy; the default of Windows), start it with consh.cmd beside
# it, from cmd or PowerShell (it runs powershell -ExecutionPolicy Bypass -File consh.ps1, or pwsh if it is there).
# Windows PowerShell 5.1 (and PowerShell 7 before 7.3) does not pass every argument to a program as it is: an empty
# argument is dropped, and one with a " in it can be split. With such arguments the script gets other $# and $1 ...
# than from Git Bash; use PowerShell 7.3 or later, or consh/bin/consh from Git Bash.

$ErrorActionPreference = 'Stop'

# The path with the case of its names as they are on the disk (as pwd -P writes it in Git Bash)
function Get-DiskPath([string]$path) {
  $item = Get-Item -LiteralPath $path -Force
  $parts = @()
  while ($item.Parent) {
    $name = (Get-ChildItem -LiteralPath $item.Parent.FullName -Force -Filter $item.Name | Select-Object -First 1).Name
    $parts = ,$name + $parts
    $item = $item.Parent
  }
  $drive = $item.FullName.TrimEnd('\').ToUpper()
  return $drive + '\' + ($parts -join '\')
}

# The checksum of POSIX cksum (CRC-32 of the bytes, then of the length)
function Get-Cksum([byte[]]$data) {
  $mask = [uint64]4294967295
  $crc = [uint64]0
  $bytes = New-Object System.Collections.Generic.List[byte]
  $bytes.AddRange($data)
  $n = [uint64]$data.Length
  while ($n -gt 0) { $bytes.Add([byte]($n -band 255)); $n = $n -shr 8 }
  foreach ($b in $bytes) {
    $crc = $crc -bxor ([uint64]$b -shl 24)
    for ($i = 0; $i -lt 8; $i++) {
      if ($crc -band 2147483648) { $crc = (($crc -shl 1) -bxor 79764919) -band $mask }
      else { $crc = ($crc -shl 1) -band $mask }
    }
  }
  return ($crc -bxor $mask)
}

# True if one of the paths, or a directory or a file whose extension is in $exts under them, is newer than $time
function Test-Newer([string[]]$paths, [string[]]$exts, [datetime]$time) {
  foreach ($p in $paths) {
    if (-not (Test-Path -LiteralPath $p)) { continue }
    $item = Get-Item -LiteralPath $p -Force
    if ($item.LastWriteTimeUtc -gt $time) { return $true }
    if ($item.PSIsContainer) {
      $newer = Get-ChildItem -LiteralPath $p -Recurse -Force |
        Where-Object { ($_.PSIsContainer -or $exts -contains $_.Extension) -and $_.LastWriteTimeUtc -gt $time } |
        Select-Object -First 1
      if ($newer) { return $true }
    }
  }
  return $false
}

# The path $path under the repository, written from the current directory when that is in the repository
# (..\..\vm\target\release\conma.exe); else $path as it is. As consh/bin/consh writes it, with backslashes
function Get-RelPath([string]$path) {
  $here = (Get-Location).ProviderPath.TrimEnd('\')
  $top = $root.TrimEnd('\')
  if (-not ($here -ieq $top -or $here.StartsWith("$top\", [StringComparison]::OrdinalIgnoreCase))) { return $path }
  if (-not $path.StartsWith("$top\", [StringComparison]::OrdinalIgnoreCase)) { return $path }
  $from = @($here.Split('\'))
  $to = @($path.Split('\'))
  $n = 0
  while ($n -lt $from.Count -and $n -lt $to.Count -and $from[$n] -ieq $to[$n]) { $n++ }
  $parts = @()
  for ($i = $n; $i -lt $from.Count; $i++) { $parts += '..' }
  for ($i = $n; $i -lt $to.Count; $i++) { $parts += $to[$i] }
  if ($parts.Count -eq 0) { return '.' }
  return $parts -join '\'
}

# If $exe is older than its sources ($sources, and the files with an extension in $exts under them; after git pull,
# for example), warn, with the command that builds it in $dir. In the interactive mode, when standard input and
# standard error are the console and cargo is there, ask whether to build it now (Enter builds it), showing how long
# its last build here took: the seconds are kept in build-seconds.<name> in the directory of bin/cache-dir, the same
# file as consh/bin/consh keeps. The output of cargo is shown only when the build fails. Without a build, or when
# it fails, the old $exe is used. Scripts, tests and pipes are not asked (so that they do not stop)
function Update-StaleBuild([string]$exe, [string]$dir, [string[]]$sources, [string[]]$exts) {
  if (-not (Test-Newer $sources $exts (Get-Item -LiteralPath $exe).LastWriteTimeUtc)) { return }
  $rexe = Get-RelPath $exe
  $rdir = Get-RelPath $dir
  if ($rdir -eq '.') { $cmd = 'cargo build --release' }
  else { $cmd = "cargo build --release --manifest-path $(Join-Path $rdir 'Cargo.toml')" }
  [Console]::Error.WriteLine("consh.ps1: warning: $rexe is older than its sources; build it with: $cmd")
  if (-not $interactive -or [Console]::IsInputRedirected -or [Console]::IsErrorRedirected -or
      -not (Get-Command cargo.exe -CommandType Application -ErrorAction SilentlyContinue)) { return }
  $rec = Join-Path $cacheDir ('build-seconds.' + [IO.Path]::GetFileNameWithoutExtension($exe))
  $last = $null
  if (Test-Path -LiteralPath $rec) { $last = (Get-Content -LiteralPath $rec -TotalCount 1) }
  if ($last) { $last = "it took $last s the last time" }
  else { $last = 'usually some seconds; longer on a slow machine or the first time' }
  # Read-Host, not [Console]::In.ReadLine(): a Ctrl-C ends the script there, but not in ReadLine, which goes on reading
  [Console]::Error.Write("consh.ps1: build it now ($last)? [Y/n] ")
  $ans = Read-Host
  if ($null -eq $ans) { $ans = 'n' }
  if (@('', 'y', 'yes') -notcontains $ans.Trim()) {
    [Console]::Error.WriteLine("consh.ps1: running the old $rexe")
    return
  }
  # The output of cargo (the warnings of the compiler and so on) only when the build fails
  [Console]::Error.WriteLine("consh.ps1: building $rexe ...")
  $out = [IO.Path]::GetTempFileName()
  $err = [IO.Path]::GetTempFileName()
  try {
    $watch = [Diagnostics.Stopwatch]::StartNew()
    $p = Start-Process -FilePath cargo.exe -ArgumentList 'build', '--release' -WorkingDirectory $dir -NoNewWindow -Wait -PassThru `
      -RedirectStandardOutput $out -RedirectStandardError $err
    $secs = [int][Math]::Floor($watch.Elapsed.TotalSeconds)
    if ($p.ExitCode -eq 0) {
      try { [IO.File]::WriteAllText($rec, "$secs`n") } catch { }
      [Console]::Error.WriteLine("consh.ps1: built $rexe in $secs s")
    } else {
      foreach ($f in @($out, $err)) { foreach ($line in [IO.File]::ReadAllLines($f)) { [Console]::Error.WriteLine($line) } }
      [Console]::Error.WriteLine("consh.ps1: the build failed; running the old $rexe")
    }
  } finally {
    Remove-Item -LiteralPath $out, $err -Force -ErrorAction SilentlyContinue
  }
}

function Exit-Usage {
  [Console]::Error.WriteLine('usage: consh.ps1 [--conma-parser] [--dump] <script> [<arg> ...]')
  exit 1
}

$consh = Get-DiskPath (Join-Path $PSScriptRoot '..')
$root = Split-Path $consh -Parent
$rest = @($args)
$mode = 'run'
$parser = $false
if ($rest.Count -gt 0 -and $rest[0] -eq '--conma-parser') { $parser = $true; $rest = @($rest | Select-Object -Skip 1) }
elseif ($rest.Count -gt 0 -and $rest[0] -eq '--python-parser') {
  [Console]::Error.WriteLine('consh.ps1: --python-parser is not taken here; use consh/bin/consh from Git Bash')
  exit 2
}
if ($rest.Count -gt 0 -and $rest[0] -eq '--dump') { $mode = 'dump'; $rest = @($rest | Select-Object -Skip 1) }
$interactive = $false
if ($rest.Count -eq 0) {
  if ($parser -or $mode -ne 'run') { Exit-Usage }
  $interactive = $true
}

$impl = if ($env:CONSH_IMPL) { $env:CONSH_IMPL } else { 'rust' }
if (@('rust', 'conma', 'reference') -notcontains $impl) {
  [Console]::Error.WriteLine("consh.ps1: CONSH_IMPL must be rust, conma or reference, not `"$impl`"")
  exit 2
}
$rust = Join-Path $consh 'rust\target\release\consh-rs.exe'
if ($impl -eq 'rust' -and ($parser -or $env:CONSH_CONMA_DATA -or -not (Test-Path -LiteralPath $rust))) { $impl = 'conma' }

# Git's usr\bin at the end of PATH, if ls.exe is not found
$oldPath = $env:PATH
$gitBin = Join-Path $env:ProgramFiles 'Git\usr\bin'
if (-not (Get-Command ls.exe -CommandType Application -ErrorAction SilentlyContinue) -and (Test-Path -LiteralPath $gitBin)) {
  $env:PATH = "$env:PATH;$gitBin"
}

try {
  if ($impl -eq 'rust') {
    if ($interactive) { & $rust interactive }
    elseif ($mode -eq 'dump') { & $rust dump $rest[0] }
    else { & $rust run @rest }
    exit $LASTEXITCODE
  }

  $vmName = if ($env:CONMA_VM) { $env:CONMA_VM } else { 'rust' }
  switch ($vmName) {
    'rust' { $vm = Join-Path $root 'vm\target\release\conma.exe' }
    'transpiled' { $vm = Join-Path $root 'vm\target-transpiled\release\conma.exe' }
    default {
      [Console]::Error.WriteLine("consh.ps1: CONMA_VM must be rust or transpiled here, not `"$vmName`"")
      exit 2
    }
  }
  $conma = Join-Path $root 'vm\target\release\conma.exe'
  foreach ($exe in @($conma, $vm) | Select-Object -Unique) {
    if (-not (Test-Path -LiteralPath $exe)) {
      [Console]::Error.WriteLine("consh.ps1: $exe is not built (cargo build --release in vm)")
      exit 2
    }
  }

  # The directory of bin/cache-dir, and that of the modules (the same, unless CONSH_MODULE_DIR is set)
  if ($env:CONMA_CACHE_DIR) { $cacheDir = $env:CONMA_CACHE_DIR }
  else {
    if ($env:XDG_CACHE_HOME) { $base = $env:XDG_CACHE_HOME }
    elseif ($env:HOME) { $base = Join-Path $env:HOME '.cache' }
    else { $base = Join-Path $env:USERPROFILE '.cache' }
    $msys = '/' + $root.Substring(0, 1).ToLower() + ($root.Substring(2) -replace '\\', '/')
    $sum = Get-Cksum ([Text.Encoding]::UTF8.GetBytes($msys))
    $cacheDir = Join-Path (Join-Path $base 'conma') $sum
  }
  if (-not (Test-Path -LiteralPath $cacheDir)) { New-Item -ItemType Directory -Force -Path $cacheDir | Out-Null }
  if ($env:CONSH_MODULE_DIR) { $dir = $env:CONSH_MODULE_DIR } else { $dir = $cacheDir }
  if (-not (Test-Path -LiteralPath $dir)) { New-Item -ItemType Directory -Force -Path $dir | Out-Null }

  $inc = Join-Path $root 'include'
  if ($impl -eq 'reference') {
    $dirs = @((Join-Path $consh 'reference'), (Join-Path $root 'sjs\include\core'), (Join-Path $consh 'include'),
      (Join-Path $root 'sjs\include'), (Join-Path $inc 'core'), (Join-Path $root 'reference'), $inc)
    $name = 'consh.reference'
  } else {
    $dirs = @((Join-Path $consh 'include'), (Join-Path $root 'sjs\include'), (Join-Path $inc 'ext'), $inc)
    $name = 'consh'
  }
  $module = Join-Path $dir "$name.module.se"
  $main = Join-Path $consh 'include\consh_main.se'
  $refMain = Join-Path $consh 'reference\consh_main.se'
  if ($impl -eq 'reference' -and (Test-Path -LiteralPath $refMain)) { $main = $refMain }
  $utf8 = New-Object System.Text.UTF8Encoding($false)
  if ($env:CONSH_CONMA_DATA) {
    $module = Join-Path $dir "$name.data.module.se"
    $main = Join-Path $dir "${name}_data_main.se"
    if (-not (Test-Path -LiteralPath $main) -or (Get-Item -LiteralPath $main).Length -eq 0) {
      $t = "$main.$PID.tmp"
      [IO.File]::WriteAllText($t, "(include `"data.se`")`n(include `"consh_main.se`")`n", $utf8)
      Move-Item -Force -LiteralPath $t -Destination $main
    }
  }

  # conma.exe older than its sources: warn (and in the interactive mode, ask whether to build it), and use it.
  # An old conma.exe does not know the subcommands added later (build and so on): frontend.module.se is in it
  $vmSrc = @((Join-Path $root 'vm\src'), (Join-Path $root 'vm\Cargo.toml'), (Join-Path $root 'vm\Cargo.lock'),
    (Join-Path $root 'vm\frontend.module.se'))
  Update-StaleBuild $conma (Join-Path $root 'vm') $vmSrc @('.rs', '.toml', '.lock', '.se')

  # Make the module again if a source or conma.exe is newer than it. The names of the files in it are written
  # from the directory above the repository, as consh/bin/consh writes them
  $stale = $true
  if ((Test-Path -LiteralPath $module) -and (Get-Item -LiteralPath $module).Length -gt 0) {
    $time = (Get-Item -LiteralPath $module).LastWriteTimeUtc
    $stale = (Test-Newer $dirs @('.se') $time) -or (Test-Newer @($conma) @() $time)
  }
  if ($stale) {
    $quoted = @('build')
    foreach ($d in $dirs) { $quoted += '--include'; $quoted += '"' + ($d -replace '\\', '/') + '"' }
    $quoted += '"' + ($main -replace '\\', '/') + '"'
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = $conma
    $psi.Arguments = $quoted -join ' '
    $psi.UseShellExecute = $false
    $psi.RedirectStandardOutput = $true
    $psi.StandardOutputEncoding = $utf8
    $p = [Diagnostics.Process]::Start($psi)
    $text = $p.StandardOutput.ReadToEnd()
    $p.WaitForExit()
    if ($p.ExitCode -ne 0) { exit $p.ExitCode }
    $above = (Split-Path $root -Parent) -replace '\\', '/'
    $text = $text.Replace('"' + $above + '/', '"')
    $t = "$module.$PID.tmp"
    [IO.File]::WriteAllText($t, $text, $utf8)
    Move-Item -Force -LiteralPath $t -Destination $module
  }

  if ($interactive) { & $vm --main $module interactive - stdin }
  else { & $vm --main $module "$mode-src" $rest[0] @rest }
  exit $LASTEXITCODE
} finally {
  $env:PATH = $oldPath
}
