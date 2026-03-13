# amctl v2 — 无头直控方案技术设计

## 1. 背景

### 1.1 v1 的瓶颈

amctl v1 所有能力都经过 Accessibility Service API，这套 API 为残障辅助设计，有固有的性能开销：

| 操作 | v1 API | 慢在哪 | 延迟 |
|------|--------|--------|------|
| 截帧 | `takeScreenshot()` | 异步回调 + vsync 等待 + SurfaceFlinger 合成 | 200-500ms |
| 点击 | `dispatchGesture()` | 构造手势轨迹 + 动画帧同步 + 完成回调 | 200-500ms |
| 滑动 | `dispatchGesture()` | 同上，还要等手势播放完 | 300-800ms |
| 文本 | `ACTION_SET_TEXT` | 跨进程 Binder + 节点查找 | 100-300ms |
| UI 树 | `getWindows()` + 递归遍历 | 跨进程逐节点 Binder 调用 | 50-200ms |

### 1.2 v2 思路

犀牛派 X1 作为无人值守设备，物理屏幕没人看。不需要虚拟 Display，直接用系统级 API 操控物理屏幕上的 App：

- **截帧**：`SurfaceControl.screenshot()` 同步直读（3-5ms），或 `screencap` 命令（50-100ms）
- **输入**：`InputManager.injectInputEvent()` 直插内核队列（1-5ms），或 `input` 命令（50ms）
- **UI 树**：复用 v1 的 Accessibility API（差异不大）
- **App 管理**：`am start` / `am force-stop`（标准 shell 命令）

### 1.3 关键决策

| 决策 | 选择 | 理由 |
|------|------|------|
| 不用 VirtualDisplay | 没人看屏幕，不需要隔离 | 省去 AOSP 修改 |
| 不改 AOSP | 纯 App 层实现 | 零刷机风险，任何 ROM 都能跑 |
| 保留 v1 作为 fallback | Accessibility 依然可用 | 兼容无 root/Shizuku 的场景 |
| Shizuku 而非 root | 获取 shell 权限 | 不需要 root，user 版也能用 |

---

## 2. 架构

### 2.1 总体架构

```
MCP 客户端 (AI)
    │
    ▼ HTTP POST /mcp + Bearer Token
┌──────────────────────────────────────────────────┐
│              McpServerService（沿用 v1）            │
│              (Android 前台服务)                     │
│                                                    │
│  Ktor HTTP Server → BearerTokenAuth → MCP SDK     │
│       │                                            │
│       ▼                                            │
│  ┌──────────┐                                      │
│  │ ToolRouter│ ── 根据可用能力自动选路径             │
│  └────┬─────┘                                      │
│       │                                            │
│  ┌────┴────────────────────┬──────────────────┐    │
│  │ v2: SystemControlProvider│ v1: Accessibility │    │
│  │ (优先使用)               │ (fallback)        │    │
│  │                         │                   │    │
│  │ ┌─ ScreenCapture ─────┐│ dispatchGesture   │    │
│  │ │ SurfaceControl      ││ takeScreenshot    │    │
│  │ │ 或 screencap        ││ nodeInfo 遍历     │    │
│  │ └─────────────────────┘│                   │    │
│  │ ┌─ InputControl ──────┐│                   │    │
│  │ │ injectInputEvent    ││                   │    │
│  │ │ 或 input 命令        ││                   │    │
│  │ └─────────────────────┘│                   │    │
│  │ ┌─ AppControl ────────┐│                   │    │
│  │ │ am start/force-stop ││                   │    │
│  │ └─────────────────────┘│                   │    │
│  └─────────────────────────┴──────────────────┘    │
│                    │                                │
│              Shizuku / shell                        │
└──────────────────────────────────────────────────────┘
```

### 2.2 权限获取方式

v2 的系统级 API 需要 shell 权限（UID 2000）。获取方式按优先级排列：

| 方式 | 需要 root？ | 需要 userdebug？ | 持久性 | 推荐 |
|------|-----------|----------------|--------|------|
| **Shizuku** | ✗ | ✗ | 重启后需重新授权 | **首选** |
| `Runtime.exec("su")` | ✓ (Magisk) | ✗ | 持久 | 备选 |
| 系统 App 签名 | ✗ | ✓ | 持久 | 备选 |
| ADB wireless 自连 | ✗ | ✗ | 不稳定 | 临时 |

**Shizuku 工作原理**：

```
一次性授权（ADB 或 root）：
  adb shell sh /sdcard/Android/data/moe.shizuku.privileged.api/start.sh

之后 App 通过 Shizuku 的 Binder：
  App 进程 → Shizuku Binder → shell 权限执行 → 返回结果
  (无 fork 开销，~1ms)
```

### 2.3 能力降级策略

```
启动时检测可用能力：

Shizuku 可用？
  ├── 是 → v2 模式：系统 API 直调（最快）
  └── 否 → root 可用？
              ├── 是 → v2 模式：su -c 执行（快）
              └── 否 → v1 模式：Accessibility（慢但通用）

运行时日志提示当前模式：
  [amctl] 使用 v2 模式 (Shizuku)，截帧 ~5ms，输入 ~2ms
  [amctl] 使用 v1 模式 (Accessibility)，截帧 ~300ms，输入 ~300ms
```

---

## 3. 核心模块设计

### 3.1 截帧

#### 3.1.1 方案对比

| 方案 | 权限 | 延迟 | 实现复杂度 |
|------|------|------|----------|
| `SurfaceControl.screenshot()` | shell (Shizuku) | **3-5ms** | 中（反射调用 hidden API） |
| `screencap` 命令 | shell | 50-100ms | 低（`Runtime.exec`） |
| `MediaProjection + ImageReader` | 一次性用户授权 | 10-30ms | 中 |
| `takeScreenshot()` (v1) | Accessibility | 200-500ms | 已实现 |

#### 3.1.2 主方案：SurfaceControl.screenshot()

```kotlin
// 通过 Shizuku 以 shell 权限调用 hidden API
object SystemScreenCapture {

    fun capture(maxWidth: Int, maxHeight: Int): Bitmap? {
        // SurfaceControl.screenshot() 是 @hide API
        // 通过反射调用，Shizuku 提供 shell 权限
        val displayToken = SurfaceControl::class.java
            .getMethod("getInternalDisplayToken")
            .invoke(null)

        val screenshot = SurfaceControl::class.java
            .getMethod("screenshot", IBinder::class.java, Rect::class.java,
                        Int::class.java, Int::class.java, Int::class.java)
            .invoke(null, displayToken, Rect(), maxWidth, maxHeight, 0) as? Bitmap

        return screenshot
    }
}
```

#### 3.1.3 降级方案：screencap 命令

```kotlin
// 不需要反射，任何有 shell 权限的方式都能用
object ShellScreenCapture {

    fun capture(): ByteArray {
        // screencap -p 输出 PNG 到 stdout
        val process = Runtime.getRuntime().exec(arrayOf("sh", "-c", "screencap -p"))
        return process.inputStream.readBytes()
    }

    // 更快的方式：直接输出 raw RGBA，跳过 PNG 编码
    fun captureRaw(): ByteArray {
        val process = Runtime.getRuntime().exec(arrayOf("sh", "-c", "screencap"))
        return process.inputStream.readBytes()
    }
}
```

### 3.2 输入注入

#### 3.2.1 主方案：InputManager.injectInputEvent()

```kotlin
// 通过 Shizuku 以 shell 权限调用
object SystemInputInjector {

    private val inputManager: Any by lazy {
        val clazz = Class.forName("android.hardware.input.InputManager")
        clazz.getMethod("getInstance").invoke(null)
    }

    private val injectMethod by lazy {
        inputManager.javaClass.getMethod(
            "injectInputEvent",
            InputEvent::class.java,
            Int::class.java
        )
    }

    fun tap(x: Float, y: Float) {
        val downTime = SystemClock.uptimeMillis()
        injectMotion(downTime, downTime, MotionEvent.ACTION_DOWN, x, y)
        injectMotion(downTime, downTime + 50, MotionEvent.ACTION_UP, x, y)
    }

    fun swipe(x1: Float, y1: Float, x2: Float, y2: Float, durationMs: Long) {
        val downTime = SystemClock.uptimeMillis()
        val steps = maxOf((durationMs / 16).toInt(), 1)

        injectMotion(downTime, downTime, MotionEvent.ACTION_DOWN, x1, y1)
        for (i in 1..steps) {
            val fraction = i.toFloat() / steps
            val x = x1 + (x2 - x1) * fraction
            val y = y1 + (y2 - y1) * fraction
            val eventTime = downTime + durationMs * i / steps
            injectMotion(downTime, eventTime, MotionEvent.ACTION_MOVE, x, y)
            Thread.sleep(16)
        }
        injectMotion(downTime, downTime + durationMs, MotionEvent.ACTION_UP, x2, y2)
    }

    fun keyEvent(keyCode: Int) {
        val now = SystemClock.uptimeMillis()
        val down = KeyEvent(now, now, KeyEvent.ACTION_DOWN, keyCode, 0)
        val up = KeyEvent(now, now + 50, KeyEvent.ACTION_UP, keyCode, 0)
        injectMethod.invoke(inputManager, down, 0) // INJECT_INPUT_EVENT_MODE_ASYNC
        injectMethod.invoke(inputManager, up, 0)
    }

    private fun injectMotion(downTime: Long, eventTime: Long, action: Int, x: Float, y: Float) {
        val event = MotionEvent.obtain(downTime, eventTime, action, x, y, 0)
        event.source = InputDevice.SOURCE_TOUCHSCREEN
        injectMethod.invoke(inputManager, event, 0)
        event.recycle()
    }
}
```

#### 3.2.2 降级方案：input 命令

```kotlin
object ShellInputInjector {

    fun tap(x: Float, y: Float) {
        Runtime.getRuntime().exec(arrayOf("sh", "-c", "input tap $x $y"))
    }

    fun swipe(x1: Float, y1: Float, x2: Float, y2: Float, durationMs: Long) {
        Runtime.getRuntime().exec(arrayOf("sh", "-c",
            "input swipe $x1 $y1 $x2 $y2 $durationMs"))
    }

    fun keyEvent(keyCode: Int) {
        Runtime.getRuntime().exec(arrayOf("sh", "-c", "input keyevent $keyCode"))
    }

    fun text(text: String) {
        Runtime.getRuntime().exec(arrayOf("sh", "-c", "input text '${text.replace("'", "'\\''")}'"))
    }
}
```

### 3.3 App 管理

```kotlin
object AppController {

    fun launch(packageName: String) {
        exec("am start -n $(cmd package resolve-activity --brief $packageName | tail -1)")
    }

    fun forceStop(packageName: String) {
        exec("am force-stop $packageName")
    }

    fun getTopActivity(): String {
        return exec("dumpsys activity activities | grep mResumedActivity")
            .trim()
    }

    fun listRunningApps(): List<String> {
        return exec("ps -A -o NAME | grep -v '^\\['")
            .lines()
            .filter { it.isNotBlank() }
    }

    private fun exec(cmd: String): String {
        val process = Runtime.getRuntime().exec(arrayOf("sh", "-c", cmd))
        return process.inputStream.bufferedReader().readText()
    }
}
```

### 3.4 UI 树

UI 树读取**沿用 v1 的 Accessibility API**，原因：

1. `UiAutomation` 需要 `DUMP` 权限且 API 不稳定
2. `uiautomator dump` 是全量 XML 解析，比 Accessibility 的增量遍历更慢
3. Accessibility 在 UI 树读取上的延迟（50-200ms）不是主要瓶颈——瓶颈在截帧和输入

如果需要进一步优化，可在 P2 阶段引入 `uiautomator` 的 `AccessibilityNodeInfoDumper` 直接调用。

### 3.5 ToolRouter — 自动选路

```kotlin
class ToolRouter @Inject constructor(
    private val shizukuProvider: ShizukuProvider,
    private val accessibilityProvider: AccessibilityServiceProvider,
) {
    enum class Mode { SYSTEM_API, SHELL_CMD, ACCESSIBILITY }

    val currentMode: Mode
        get() = when {
            shizukuProvider.isAvailable() -> Mode.SYSTEM_API
            shellAvailable() -> Mode.SHELL_CMD
            else -> Mode.ACCESSIBILITY
        }

    // 截帧
    suspend fun captureScreen(maxWidth: Int, maxHeight: Int): ScreenshotData =
        when (currentMode) {
            Mode.SYSTEM_API -> systemScreenCapture.capture(maxWidth, maxHeight)
            Mode.SHELL_CMD -> shellScreenCapture.capture()
            Mode.ACCESSIBILITY -> accessibilityProvider.takeScreenshot()
        }

    // 输入
    suspend fun tap(x: Float, y: Float) =
        when (currentMode) {
            Mode.SYSTEM_API -> systemInputInjector.tap(x, y)
            Mode.SHELL_CMD -> shellInputInjector.tap(x, y)
            Mode.ACCESSIBILITY -> accessibilityProvider.dispatchGesture(tapGesture(x, y))
        }

    // App 管理（仅 v2）
    fun launchApp(packageName: String) = appController.launch(packageName)
    fun forceStopApp(packageName: String) = appController.forceStop(packageName)
}
```

---

## 4. MCP 工具变更

### 4.1 现有工具（内部提速，接口不变）

所有 17 个 v1 工具**接口完全不变**——参数、返回值、工具名都一样。LLM 端零修改。

内部实现通过 ToolRouter 自动选择最快的路径：

| 工具 | v1 内部实现 | v2 内部实现 | 提速 |
|------|-----------|-----------|------|
| `amctl_get_screen_state` | `takeScreenshot()` | `SurfaceControl.screenshot()` | 40-100x |
| `amctl_tap` | `dispatchGesture()` | `injectInputEvent()` | 40-100x |
| `amctl_swipe` | `dispatchGesture()` | `injectInputEvent()` 序列 | 6-8x |
| `amctl_press_back` | `performGlobalAction()` | `injectKeyEvent(BACK)` | 2-5x |
| 其他工具 | 无变化 | 无变化 | — |

### 4.2 新增工具

| 工具 | 描述 | 参数 | 需要 |
|------|------|------|------|
| `amctl_launch_app` | 启动 App | `package_name` | shell |
| `amctl_force_stop_app` | 停止 App | `package_name` | shell |
| `amctl_get_top_activity` | 获取当前 Activity | — | shell |

### 4.3 工具总数

v1 的 17 个 + 新增 3 个 = **20 个**。

---

## 5. Shizuku 集成

### 5.1 依赖

```toml
# gradle/libs.versions.toml
[versions]
shizuku = "13.1.5"

[libraries]
shizuku-api = { module = "dev.rikka.shizuku:api", version.ref = "shizuku" }
shizuku-provider = { module = "dev.rikka.shizuku:provider", version.ref = "shizuku" }
```

### 5.2 初始化

```xml
<!-- AndroidManifest.xml -->
<provider
    android:name="rikka.shizuku.ShizukuProvider"
    android:authorities="${applicationId}.shizuku"
    android:multiprocess="false"
    android:enabled="true"
    android:exported="true"
    android:permission="android.permission.INTERACT_ACROSS_USERS_FULL" />
```

### 5.3 权限请求

```kotlin
class ShizukuProvider @Inject constructor() {

    fun isAvailable(): Boolean =
        Shizuku.pingBinder() && Shizuku.checkSelfPermission() == PackageManager.PERMISSION_GRANTED

    fun requestPermission(activity: Activity) {
        if (Shizuku.shouldShowRequestPermissionRationale()) {
            // 提示用户
        }
        Shizuku.requestPermission(REQUEST_CODE)
    }

    // 以 shell 身份执行命令
    fun exec(command: String): String {
        val process = Shizuku.newProcess(arrayOf("sh", "-c", command), null, null)
        return process.inputStream.bufferedReader().readText()
    }
}
```

### 5.4 首次设置

犀牛派 X1 上一次性操作：

```bash
# 1. 安装 Shizuku App
adb install shizuku.apk

# 2. 启动 Shizuku 服务
adb shell sh /sdcard/Android/data/moe.shizuku.privileged.api/start.sh

# 3. 安装 amctl
adb install amctl.apk

# 4. amctl 启动时自动请求 Shizuku 权限
```

设备重启后需要重新执行步骤 2。可通过 init.d 脚本或 Tasker 自动化。

---

## 6. 性能预期

### 6.1 延迟对比

| 操作 | v1 (Accessibility) | v2 Shell 命令 | v2 Shizuku | 提升倍数 |
|------|-------------------|-------------|-----------|---------|
| 截帧 | 200-500ms | 50-100ms | **3-5ms** | 40-100x |
| tap | 200-500ms | 50ms | **1-5ms** | 40-100x |
| swipe | 300-800ms | 100ms | **50-100ms** | 6-8x |
| 文本输入 | 100-300ms | 50ms | **1-5ms** | 20-60x |
| UI 树 | 50-200ms | 同 | **同** | — |
| 全流程 | 1-3s | 300-500ms | **50-200ms** | 5-15x |

### 6.2 与定制 ROM 方案对比

| | 无头直控 (Shizuku) | 定制 ROM |
|--|--|--|
| 全流程延迟 | 50-200ms | 50-200ms |
| 截帧 | 3-5ms | 1-3ms |
| 输入 | 1-5ms | 1-5ms |
| 并行控制 | 1 个 App | 多个 (VirtualDisplay) |
| 静默执行 | ✗（占物理屏） | ✓ |
| 开发周期 | 1-2 周 | 8-14 周 |

**结论**：对于无人值守的犀牛派场景，无头直控的性能与定制 ROM 几乎相同，开发成本低一个数量级。

---

## 7. 目录结构变更

在 v1 基础上新增以下文件：

```
app/src/main/kotlin/com/example/amctl/
├── services/
│   ├── system/                          ← 新增
│   │   ├── ToolRouter.kt                ← 路由：v2 / v1 自动选择
│   │   ├── ShizukuProvider.kt           ← Shizuku 权限管理
│   │   ├── SystemScreenCapture.kt       ← SurfaceControl.screenshot()
│   │   ├── ShellScreenCapture.kt        ← screencap 命令
│   │   ├── SystemInputInjector.kt       ← InputManager.injectInputEvent()
│   │   ├── ShellInputInjector.kt        ← input 命令
│   │   └── AppController.kt            ← am start / force-stop
│   ├── accessibility/                   ← 不变（v1 fallback）
│   ├── screencapture/                   ← 不变（v1 fallback）
│   └── mcp/                             ← 不变
├── mcp/
│   └── tools/
│       ├── AppTools.kt                  ← 新增：launch_app, force_stop, top_activity
│       └── ...                          ← 现有工具内部改用 ToolRouter
└── ui/
    └── components/
        └── PermissionsSection.kt        ← 修改：增加 Shizuku 状态显示
```

---

## 8. 与 v1 的兼容性

| 场景 | 行为 |
|------|------|
| 有 Shizuku | v2 模式，系统 API 直调，最快 |
| 有 root 无 Shizuku | v2 模式，shell 命令，较快 |
| 无 root 无 Shizuku | v1 模式，Accessibility，正常可用 |
| Shizuku 运行中断开 | 自动降级到 v1，日志警告 |
| 工具接口 | 完全不变，LLM 端零修改 |

---

## 9. 风险与缓解

| 风险 | 可能性 | 缓解 |
|------|--------|------|
| Shizuku 重启后失效 | 高 | init.d 脚本自动启动，或 ADB over WiFi 自连 |
| `SurfaceControl` API 在不同 Android 版本签名不同 | 中 | 多版本反射适配，降级到 `screencap` |
| `injectInputEvent` 被 SELinux 拦截 | 低 | Shizuku 以 shell 身份运行，shell 域有注入权限 |
| 部分 App 检测 input injection | 低 | 与 v1 的 `dispatchGesture` 风险相同 |
| 反射调用的 hidden API 被移除 | 低 | Android CTS 保证 `screencap`/`input` 命令稳定存在 |
