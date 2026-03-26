; Kalam custom NSIS hooks — merged into Tauri's installer.nsi (see tauri.conf.json → bundle.windows.nsis.installerHooks).
; Removes user data outside Tauri's bundle-id dirs: %USERPROFILE%\.kalam and %LOCALAPPDATA%\Kalam (ProjectDirs for models/sidecars).
; Runs only when "Delete the application data" was checked and not during /UPDATE (same intent as Tauri's built-in cleanup).

!macro NSIS_HOOK_POSTUNINSTALL
  StrCmp $DeleteAppDataCheckboxState 1 kalam_cleanup
  Goto skip_kalam_extra
  kalam_cleanup:
  StrCmp $UpdateMode 1 skip_kalam_extra
  SetShellVarContext current
  ; Primary config, DB, history, logs (get_kalam_dir).
  RmDir /r /REBOOTOK "$PROFILE\.kalam"
  ; Models, sidecars, VAD (directories::ProjectDirs data_local_dir on Windows).
  RmDir /r /REBOOTOK "$LOCALAPPDATA\Kalam"
  skip_kalam_extra:
!macroend
