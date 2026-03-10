@echo off
setlocal enabledelayedexpansion

set "ROOT_DIR=%~dp0"

where cargo >nul 2>nul
if errorlevel 1 (
  if exist "%USERPROFILE%\.cargo\env.bat" (
    call "%USERPROFILE%\.cargo\env.bat"
  )
)

where cargo >nul 2>nul
if errorlevel 1 (
  echo cargo not found. Install Rust (rustup) first.
  exit /b 1
)

set "INSTALL_DIR=%USERPROFILE%\.cargo\bin"
if not exist "%INSTALL_DIR%" (
  mkdir "%INSTALL_DIR%"
)

pushd "%ROOT_DIR%"
cargo build --release
copy /Y "target\release\todo.exe" "%INSTALL_DIR%\todo.exe" >nul
popd

echo Installed todo to %INSTALL_DIR%\todo.exe
set "TASK_NAME=TodoReminder"
schtasks /Query /TN "%TASK_NAME%" >nul 2>nul
if not errorlevel 1 (
  schtasks /Delete /TN "%TASK_NAME%" /F >nul
)
schtasks /Create /SC MINUTE /MO 1 /TN "%TASK_NAME%" /TR "\"%INSTALL_DIR%\todo.exe\" notify" /F >nul
if errorlevel 1 (
  echo Failed to create Task Scheduler job. You may need to run as Administrator.
) else (
  echo Enabled Task Scheduler job %TASK_NAME%
)
endlocal
