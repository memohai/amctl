# Auto Fish server notes

Only what is currently used.

```text
af (CLI)
  -> HTTP + Bearer Token
RestServerService (foreground service)
  -> RestServer (Ktor)
     - /api/tap (coordinate tap)
     - /api/nodes/tap (semantic node tap by text/desc/resource id)
     - /api/screen/refs (clickable refs + refVersion)
     - /api/nodes/tap by=ref (server-side ref alias mapping: exact token first, identity token fallback only when unique)
  -> ToolRouter
      -> v2: system/shizuku/shell
      -> v1: accessibility (fallback)
  -> Android device
```

## Android APIs by purpose

### Position / bounds

- `AccessibilityNodeInfo.getBoundsInScreen(Rect)`
- `AccessibilityWindowInfo` (window metadata)

### Text / description

- `AccessibilityNodeInfo.getText()`
- `AccessibilityNodeInfo.getContentDescription()`
- `AccessibilityNodeInfo.getViewIdResourceName()`

### Screenshot

- v2: system/shizuku/shell path (preferred)
- v1: `AccessibilityService.takeScreenshot()` (fallback)

### Tap / swipe / back / home / input

- v2: system/shizuku/shell path (preferred)
- v1:
  - `AccessibilityService.dispatchGesture()`
  - `AccessibilityService.performGlobalAction()`
  - `AccessibilityNodeInfo` node actions / text operations

### On-screen overlay marks

- `WindowManager`
- `TYPE_ACCESSIBILITY_OVERLAY`
- Custom `View` for boxes and labels
