; PhantomEar NSIS Installer Hooks
; Checks for VC++ Runtime and prompts user to install if missing

!macro NSIS_HOOK_POSTINSTALL
  ; Check if VC++ Runtime DLLs exist in System32
  IfFileExists "$SYSDIR\msvcp140.dll" CheckSecondDll 0
    Goto NeedVCRedist

  CheckSecondDll:
    IfFileExists "$SYSDIR\vcruntime140.dll" SkipVCRedist 0

  NeedVCRedist:
    ; Prompt user to download VC++ Runtime
    MessageBox MB_YESNO|MB_ICONQUESTION "PhantomEar requires Microsoft Visual C++ Runtime which is not installed.$\n$\nWould you like to open the download page?" IDNO SkipVCRedist
    ExecShell "open" "https://aka.ms/vs/17/release/vc_redist.x64.exe"

  SkipVCRedist:
!macroend
