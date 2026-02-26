; PhantomEar NSIS installer hooks
; Checks for and installs Visual C++ 2015-2022 x64 Redistributable if missing.
; The inetc plugin is bundled with Tauri's NSIS build environment.

!macro customInstall
  ; Only needed on 64-bit Windows (all PhantomEar builds are x64)
  ${If} ${RunningX64}
    ; Check primary registry location for VC++ 2015-2022 x64
    ClearErrors
    ReadRegStr $0 HKLM "SOFTWARE\Microsoft\VisualStudio\14.0\VC\Runtimes\x64" "Installed"
    ${If} $0 != "1"
      ; Also try WOW6432Node (32-bit registry view)
      ReadRegStr $0 HKLM "SOFTWARE\WOW6432Node\Microsoft\VisualStudio\14.0\VC\Runtimes\x64" "Installed"
    ${EndIf}

    ${If} $0 != "1"
      DetailPrint "Visual C++ 2015-2022 Runtime not found — downloading..."
      inetc::get \
        /CAPTION "Downloading Visual C++ Runtime" \
        /BANNER "PhantomEar requires the Visual C++ Runtime. Downloading..." \
        "https://aka.ms/vs/17/release/vc_redist.x64.exe" \
        "$TEMP\vc_redist.x64.exe" /END
      Pop $1 ; download result

      ${If} $1 == "OK"
        DetailPrint "Installing Visual C++ Runtime..."
        ExecWait '"$TEMP\vc_redist.x64.exe" /install /quiet /norestart' $2
        Delete "$TEMP\vc_redist.x64.exe"
        ${If} $2 != 0
          ; Exit code 3010 = success, reboot required — treat as success
          ${If} $2 != 3010
            MessageBox MB_ICONEXCLAMATION \
              "Visual C++ Runtime installation failed (code $2).$\n$\nPlease install it manually:$\nhttps://aka.ms/vs/17/release/vc_redist.x64.exe$\n$\nThen restart PhantomEar."
          ${EndIf}
        ${EndIf}
      ${Else}
        ; Download failed — show manual install prompt
        MessageBox MB_ICONEXCLAMATION|MB_YESNO \
          "Could not download the Visual C++ Runtime automatically.$\n$\nPhantomEar requires this to run. Would you like to open the download page?" \
          IDYES openVcRedist IDNO skipVcRedist
        openVcRedist:
          ExecShell "open" "https://aka.ms/vs/17/release/vc_redist.x64.exe"
        skipVcRedist:
      ${EndIf}
    ${EndIf}
  ${EndIf}
!macroend
