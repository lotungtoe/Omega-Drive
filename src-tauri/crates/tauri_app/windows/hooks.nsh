!macro NSIS_HOOK_POSTINSTALL
  DetailPrint "Downloading mpv.dll..."
  NSISdl::download /TIMEOUT=30000 "https://github.com/lotungtoe/Omega-Drive/releases/download/deps-v1/mpv.dll" "$INSTDIR\mpv.dll"
  Pop $0
  ${If} $0 != 0
    DetailPrint "Warning: mpv.dll download failed"
  ${Else}
    CopyFiles /SILENT "$INSTDIR\mpv.dll" "$INSTDIR\mpv-1.dll"
    DetailPrint "mpv.dll installed"
  ${EndIf}

  DetailPrint "Downloading libmpv-2.dll..."
  NSISdl::download /TIMEOUT=30000 "https://github.com/lotungtoe/Omega-Drive/releases/download/deps-v1/libmpv-2.dll" "$INSTDIR\libmpv-2.dll"
  Pop $0
  ${If} $0 != 0
    DetailPrint "Warning: libmpv-2.dll download failed"
  ${Else}
    DetailPrint "libmpv-2.dll installed"
  ${EndIf}
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  Delete "$INSTDIR\mpv.dll"
  Delete "$INSTDIR\mpv-1.dll"
  Delete "$INSTDIR\libmpv-2.dll"
!macroend
