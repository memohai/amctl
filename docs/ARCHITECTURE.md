# amctl server notes

Only what is currently used.

```text
amc (CLI)
  -> HTTP + Bearer Token
RestServerService (foreground service)
  -> RestServer (Ktor)
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
