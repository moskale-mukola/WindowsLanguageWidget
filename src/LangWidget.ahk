#Requires AutoHotkey v2.0
#SingleInstance Force
Persistent

; =========================================================
;  LangWidget — keyboard layout widget with a change lock
;  Click language -> switch active window's layout
;  Click 🔓/🔒    -> lock/unlock layout changes
;  Click 📍/📌    -> lock/unlock widget movement (drag by background)
;  Tray           -> Autostart / Reload / Exit
; =========================================================

; ---------- Appearance ----------
W := 190, H := 74, RADIUS := 18
BG     := "1E1E1E"     ; card background color
ACCENT := "FF6B6B"     ; language color when layout is locked

iniPath    := A_ScriptDir "\LangWidget.ini"
startupLnk := A_Startup "\LangWidget.lnk"

; ---------- State ----------
layoutLocked := IniRead(iniPath, "state", "layoutLocked", "0") = "1"
posLocked    := IniRead(iniPath, "state", "posLocked", "0") = "1"
lockedTarget := GetForegroundHKL()

; ---------- Window ----------
g := Gui("+AlwaysOnTop -Caption +ToolWindow +E0x08000000")  ; E0x08000000 = doesn't steal focus
g.BackColor := BG
g.MarginX := 0, g.MarginY := 0

langCtrl := g.AddText("x16 y15 w70 h44 Center +0x200 +0x100 cFFFFFF", "EN")
langCtrl.SetFont("s26 bold", "Segoe UI")
langCtrl.OnEvent("Click", SwitchLang)

lockCtrl := g.AddText("x92 y15 w52 h44 Center +0x200 +0x100", "🔓")
lockCtrl.SetFont("s22", "Segoe UI Emoji")
lockCtrl.OnEvent("Click", ToggleLock)

posCtrl := g.AddText("x160 y8 w22 h20 Center +0x200 +0x100", "📍")
posCtrl.SetFont("s11", "Segoe UI Emoji")
posCtrl.OnEvent("Click", TogglePosLock)

startX := IniRead(iniPath, "pos", "x", 1000)
startY := IniRead(iniPath, "pos", "y", 400)
g.Show(Format("x{} y{} w{} h{} NoActivate", startX, startY, W, H))
WinSetRegion(Format("0-0 W{} H{} R{}-{}", W, H, RADIUS, RADIUS), g.Hwnd)  ; rounded corners

UpdateDisplay()

; ---------- Timers ----------
SetTimer(EnforceLock, 150)     ; keep the locked layout in place
SetTimer(UpdateDisplay, 250)   ; refresh label (e.g. after Win+Space)

; ---------- Dragging ----------
OnMessage(0x201, WM_LBUTTONDOWN)      ; WM_LBUTTONDOWN
OnMessage(0x232, (*) => SaveState())  ; WM_EXITSIZEMOVE — save position after a move

WM_LBUTTONDOWN(wParam, lParam, msg, hwnd) {
    global posLocked, g
    if posLocked
        return
    if (hwnd = g.Hwnd)                 ; background only, not the controls
        PostMessage(0xA1, 2, 0, , "ahk_id " g.Hwnd)  ; drag via fake "titlebar" click
}

; ---------- Tray ----------
A_TrayMenu.Delete()
A_TrayMenu.Add("Autostart", ToggleAutostart)
A_TrayMenu.Add()
A_TrayMenu.Add("Reload", (*) => Reload())
A_TrayMenu.Add("Exit", (*) => ExitApp())
RefreshTray()

; =========================================================
SwitchLang(*) {
    global layoutLocked, lockedTarget
    layouts := GetLayoutList()
    if !layouts.Length
        return
    cur := GetForegroundHKL()
    idx := 0
    for i, h in layouts
        if (h = cur) {
            idx := i
            break
        }
    nextHkl := (idx = 0 || idx = layouts.Length) ? layouts[1] : layouts[idx + 1]
    ApplyLayout(nextHkl)
    if layoutLocked
        lockedTarget := nextHkl        ; manual switch updates the lock target too
    UpdateDisplay()
}

ToggleLock(*) {
    global layoutLocked, lockedTarget
    layoutLocked := !layoutLocked
    if layoutLocked
        lockedTarget := GetForegroundHKL()
    UpdateDisplay()
    SaveState()
}

TogglePosLock(*) {
    global posLocked
    posLocked := !posLocked
    UpdateDisplay()
    SaveState()
}

EnforceLock() {
    global layoutLocked, lockedTarget
    if !layoutLocked || !lockedTarget
        return
    cur := GetForegroundHKL()
    if (cur && (cur & 0xFFFF) != (lockedTarget & 0xFFFF))
        ApplyLayout(lockedTarget)      ; Alt+Shift changed it — switch back
}

UpdateDisplay() {
    global langCtrl, lockCtrl, posCtrl, layoutLocked, posLocked, lockedTarget, ACCENT
    hkl := layoutLocked ? lockedTarget : GetForegroundHKL()
    langCtrl.Text := LangAbbr(hkl)
    langCtrl.SetFont(layoutLocked ? "c" ACCENT : "cFFFFFF")
    lockCtrl.Text := layoutLocked ? "🔒" : "🔓"
    posCtrl.Text  := posLocked ? "📌" : "📍"
}

; ---------- WinAPI helpers ----------
GetForegroundHKL() {
    hwnd := DllCall("GetForegroundWindow", "Ptr")
    if !hwnd
        return 0
    tid := DllCall("GetWindowThreadProcessId", "Ptr", hwnd, "Ptr", 0, "UInt")
    return DllCall("GetKeyboardLayout", "UInt", tid, "Ptr")
}

ApplyLayout(hkl) {
    hwnd := DllCall("GetForegroundWindow", "Ptr")
    if hwnd
        PostMessage(0x50, 0, hkl, , "ahk_id " hwnd)  ; WM_INPUTLANGCHANGEREQUEST
}

GetLayoutList() {
    n := DllCall("GetKeyboardLayoutList", "Int", 0, "Ptr", 0, "Int")
    if !n
        return []
    buf := Buffer(n * A_PtrSize, 0)
    DllCall("GetKeyboardLayoutList", "Int", n, "Ptr", buf)
    list := []
    Loop n
        list.Push(NumGet(buf, (A_Index - 1) * A_PtrSize, "Ptr"))
    return list
}

LangAbbr(hkl) {
    if !hkl
        return "--"
    lcid := hkl & 0xFFFF
    buf := Buffer(20, 0)
    ; 0x59 = LOCALE_SISO639LANGNAME -> "en" / "uk" / "ru"
    if DllCall("GetLocaleInfo", "UInt", lcid, "UInt", 0x59, "Ptr", buf, "Int", 10)
        return StrUpper(StrGet(buf))
    return Format("{:04X}", lcid)
}

; ---------- Persistence ----------
SaveState() {
    global g, iniPath, layoutLocked, posLocked
    g.GetPos(&x, &y)
    IniWrite(x, iniPath, "pos", "x")
    IniWrite(y, iniPath, "pos", "y")
    IniWrite(layoutLocked ? 1 : 0, iniPath, "state", "layoutLocked")
    IniWrite(posLocked ? 1 : 0, iniPath, "state", "posLocked")
}

; ---------- Autostart ----------
ToggleAutostart(*) {
    global startupLnk
    if FileExist(startupLnk) {
        FileDelete(startupLnk)
        TrayTip("Autostart disabled", "LangWidget")
    } else {
        FileCreateShortcut(A_AhkPath, startupLnk, A_ScriptDir, '"' A_ScriptFullPath '"')
        TrayTip("Autostart enabled", "LangWidget")
    }
    RefreshTray()
}

RefreshTray() {
    global startupLnk
    if FileExist(startupLnk)
        A_TrayMenu.Check("Autostart")
    else
        A_TrayMenu.Uncheck("Autostart")
}
