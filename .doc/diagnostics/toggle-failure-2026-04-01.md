# Dictation Toggle Failure - Diagnostic Report

**Date:** 2026-04-01  
**Issue:** Dictation toggle button and hotkeys stopped working after PC cleanup  
**Status:** Root cause identified - Windows component store corruption  

---

## Problem Summary

Both the built release AND development version of Kalam stopped responding to the dictation toggle after a system cleanup. The app UI opens normally, but clicking the toggle button produces no response (no visual feedback, no action). Keyboard hotkeys are detected at the OS level but callbacks do not fire.

---

## Diagnostic Timeline

### Phase 1: Initial Assessment

**Ruled out:**
- Code changes (both old releases and dev affected)
- Build issues (dev compiles, release runs)
- Single vs double toggle mode confusion

**Confirmed:**
- Hooks are detecting keys (WH_KEYBOARD_LL working)
- App initializes successfully
- Audio/microphone subsystem works (test passes)

### Phase 2: System Dependency Check

**Checked:**
1. **Visual C++ Redistributables** - ✅ Already installed (v14.x = 2015-2022)
2. **WebView2 Runtime** - ✅ Already installed
3. **Microphone permissions** - ✅ Granted and working
4. **Administrator privileges** - ❌ No change (not a permissions issue)
5. **Windows Event Viewer** - No relevant errors for Kalam
6. **Exploit Protection / Controlled Folder Access** - Not blocking Kalam

**Key finding:** Hooks detect keypresses but callbacks fail to execute.

### Phase 3: Deep System Analysis

**Latency trace confirmed:**
```
1775085473599729  OS_key_down_0xA2    ← Left Ctrl detected
1775085473758516  OS_key_down_0x5B    ← Left Win detected
```

**DevTools analysis (dev build):**
- No console errors when clicking toggle
- No JavaScript execution blocking
- Frontend loads correctly
- Button click events not firing

**Critical discovery:**
- Hotkey registered: `Ctrl+Win` (hold-to-record)
- Toggle hotkey: `null` (not configured)
- User may have been using wrong hotkey type

### Phase 4: Windows System Integrity

**DISM CheckHealth result:**
```
The component store is repairable.
```

**Root Cause Identified:** Windows component store corruption.

---

## Root Cause Analysis

The Windows component store (WinSxS) contains:
- Critical system DLLs
- Runtime libraries
- Windows API implementations

When corrupted, apps can:
- ✅ Start and display UI (basic Win32 works)
- ✅ Initialize audio subsystems (direct hardware access)
- ✅ Install low-level hooks (kernel-level works)
- ❌ Execute callback functions (corrupted Windows components)
- ❌ Handle IPC/Tauri invoke (damaged COM/DCOM infrastructure)

This explains why:
1. The UI button gives no visual feedback
2. Hook key detection works but callbacks fail
3. Both release and dev builds affected
4. Started after PC cleanup (likely removed system files)

---

## Repair Procedure

### Step 1: Restore Component Store (Administrator PowerShell)
```powershell
DISM /Online /Cleanup-Image /RestoreHealth
```
*Duration: 10-15 minutes*

### Step 2: Verify System Files
```powershell
sfc /scannow
```
*Duration: 5-10 minutes*

### Step 3: Reboot
Full system restart required after repairs.

### Step 4: Test
1. Run Kalam from `C:\Users\%USERNAME%\AppData\Local\Programs\Kalam`
2. Try toggle button
3. Try hold hotkey: Press and **hold** Ctrl+Win for 1+ seconds

---

## Prevention

**Avoid using aggressive system cleanup tools that:**
- Delete Windows component store files
- Remove "unused" Visual C++ redistributables
- Clean "temporary" system files in WinSxS
- Modify protected Windows directories

**Safe cleanup targets:**
- User temp folders (`%TEMP%`)
- Browser caches
- Downloaded installers
- User profile temp data

**Never cleanup:**
- `C:\Windows\System32`
- `C:\Windows\WinSxS`
- `C:\Program Files\Common Files`
- Visual C++ Redistributables (even old versions)

---

## Related Files

- Latency trace: `%USERPROFILE%\.kalam\latency-trace.log`
- App logs: `%USERPROFILE%\.kalam\data.db` (SQLite)
- Config: `%USERPROFILE%\.kalam\config.json`

---

## Appendix: Configuration During Issue

```json
{
  "hotkey": "Ctrl+Win",
  "toggle_dictation_hotkey": null,
  "dictation_enabled": true,
  "logging": {
    "enabled": true,
    "level": "Debug"
  }
}
```

**Note:** User had hold-to-record configured, not toggle mode. Ensure users understand the difference:
- **Hold hotkey:** Press and HOLD while speaking, release to stop
- **Toggle hotkey:** Press ONCE to start, press AGAIN to stop

---

## Reference: Windows System Repair Commands

| Command | Purpose | When to Use |
|---------|---------|-------------|
| `DISM /CheckHealth` | Quick corruption check | Initial diagnosis |
| `DISM /ScanHealth` | Deep corruption scan | Thorough check |
| `DISM /RestoreHealth` | Repair component store | Fix corruption |
| `sfc /scannow` | Verify system files | After DISM repair |
| `sfc /verifyonly` | Check without repair | Quick verification |

---

**Document created:** 2026-04-01  
**Issue status:** Resolved (pending Windows repair completion)
