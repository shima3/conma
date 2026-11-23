@echo off
rem consh.cmd: start consh.ps1 (ConSh from PowerShell on Windows) from cmd or PowerShell, whatever the execution
rem policy of PowerShell is (a .cmd file is not under it).
rem   consh.cmd [--conma-parser] [--dump] <script> [<arg> ...]
rem   consh.cmd                 interactive mode
rem It uses PowerShell 7 (pwsh) if it is on PATH, as it passes the arguments to programs as they are (7.3 and later);
rem else Windows PowerShell 5.1, which drops empty arguments and can split one with a " in it (consh.ps1).
rem After a Ctrl-C in interactive mode, cmd asks "Terminate batch job (Y/N)?" when ConSh has ended (cmd gets the
rem Ctrl-C too; no .cmd file can keep it from asking). Y and N end the same, as ConSh has ended already.
rem Start consh.ps1 itself to have no question.
setlocal
where pwsh >nul 2>nul
if %ERRORLEVEL% equ 0 (
  pwsh -NoProfile -ExecutionPolicy Bypass -File "%~dp0consh.ps1" %*
) else (
  powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0consh.ps1" %*
)
exit /b %ERRORLEVEL%
