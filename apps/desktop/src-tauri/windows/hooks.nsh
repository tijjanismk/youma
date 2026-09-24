; Installation hors ligne : WebView2 est embarqué (offlineInstaller).
; Exception pare-feu pour que les téléphones du réseau local joignent le poste central (mode B).
!macro NSIS_HOOK_POSTINSTALL
  nsExec::Exec 'netsh advfirewall firewall delete rule name="Youma"'
  nsExec::Exec 'netsh advfirewall firewall add rule name="Youma" dir=in action=allow program="$INSTDIR\youma-desktop.exe" enable=yes profile=private,domain'
  CreateDirectory "$COMMONAPPDATA\Youma"
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  nsExec::Exec 'netsh advfirewall firewall delete rule name="Youma"'
  ; Les données (%PROGRAMDATA%\Youma) ne sont jamais supprimées par la désinstallation.
!macroend
