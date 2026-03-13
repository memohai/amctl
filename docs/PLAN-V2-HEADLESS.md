# amctl v2 — 无头直控实施计划

本文档是 amctl v2（无头直控方案）的逐步实施计划。
请先阅读 [DESIGN-V2-HEADLESS.md](DESIGN-V2-HEADLESS.md) 了解架构和技术选型。

v2 在 v1 基础上扩展，不改动现有代码的对外接口，仅替换内部实现路径。

---

## 前提条件

1. **v1 功能完整可用** — 所有 17 个 MCP 工具通过测试
2. **Shizuku App APK** — 从 GitHub 下载 [Shizuku releases](https://github.com/RikkaApps/Shizuku/releases)
3. **目标设备** — 犀牛派 X1 或 Android 模拟器（先在模拟器上开发验证）

不需要：BSP 源码、userdebug ROM、root、编译服务器、改 AOSP。

---

## 阶段 P0：Shizuku 集成（2-3 天）

**目标**：App 能通过 Shizuku 获取 shell 权限并执行命令。

### 任务 P0.1：添加 Shizuku 依赖

| 文件 | 操作 | 说明 |
|------|------|------|
| `gradle/libs.versions.toml` | 修改 | 添加 `shizuku = "13.1.5"`，`shizuku-api`、`shizuku-provider` |
| `app/build.gradle.kts` | 修改 | 添加 Shizuku 依赖 |
| `app/src/main/AndroidManifest.xml` | 修改 | 添加 `ShizukuProvider` 声明 |

```toml
# libs.versions.toml 新增
[versions]
shizuku = "13.1.5"

[libraries]
shizuku-api = { module = "dev.rikka.shizuku:api", version.ref = "shizuku" }
shizuku-provider = { module = "dev.rikka.shizuku:provider", version.ref = "shizuku" }
```

**DoD**：编译通过。

### 任务 P0.2：ShizukuProvider 封装

| 文件 | 操作 | 说明 |
|------|------|------|
| `app/.../services/system/ShizukuProvider.kt` | 新增 | 接口：`isAvailable()`, `requestPermission()`, `exec(command): String` |
| `app/.../services/system/ShizukuProviderImpl.kt` | 新增 | Shizuku API 封装实现 |
| `app/.../di/AppModule.kt` | 修改 | 注册 ShizukuProvider 绑定 |

关键方法：

```kotlin
interface ShizukuProvider {
    fun isAvailable(): Boolean
    fun exec(command: String): String
    fun execBytes(command: String): ByteArray
}
```

**DoD**：在模拟器上安装 Shizuku + amctl，`ShizukuProvider.exec("id")` 返回 `uid=2000(shell)`。

### 任务 P0.3：UI 增加 Shizuku 状态

| 文件 | 操作 | 说明 |
|------|------|------|
| `app/.../ui/components/PermissionsSection.kt` | 新增 | 显示 Shizuku 状态（未安装/未授权/已授权）+ 授权按钮 |
| `app/.../ui/screens/HomeScreen.kt` | 修改 | 在无障碍权限下方增加 Shizuku 权限区域 |

**DoD**：UI 正确显示 Shizuku 连接状态，点击按钮可触发授权。

---

## 阶段 P1：系统级截帧（2-3 天）

**目标**：替换 `takeScreenshot()` 为更快的截帧方式。

### 任务 P1.1：Shell 截帧

| 文件 | 操作 | 说明 |
|------|------|------|
| `app/.../services/system/ShellScreenCapture.kt` | 新增 | 通过 `screencap` 命令截帧 |

```kotlin
class ShellScreenCapture @Inject constructor(
    private val shizukuProvider: ShizukuProvider,
) {
    fun capture(): ByteArray =
        shizukuProvider.execBytes("screencap -p")

    fun captureToFile(path: String) =
        shizukuProvider.exec("screencap -p $path")
}
```

**DoD**：通过 Shizuku 调用 `screencap` 成功返回 PNG 数据。

### 任务 P1.2：SurfaceControl 截帧（可选优化）

| 文件 | 操作 | 说明 |
|------|------|------|
| `app/.../services/system/SystemScreenCapture.kt` | 新增 | 通过反射调用 `SurfaceControl.screenshot()` |

需要处理 Android 版本差异：
- Android 14: `SurfaceControl.screenshot(displayToken, ...)`
- Android 12-13: 参数签名略有不同

如果反射失败，降级到 `ShellScreenCapture`。

**DoD**：`SurfaceControl.screenshot()` 成功返回 Bitmap，耗时 <10ms。反射失败时自动降级。

### 任务 P1.3：性能对比测试

在模拟器上对比三种截帧方式的延迟：

| 方式 | 预期延迟 |
|------|---------|
| `SurfaceControl.screenshot()` via Shizuku | <10ms |
| `screencap -p` via Shizuku | 50-100ms |
| `takeScreenshot()` via Accessibility (v1) | 200-500ms |

**DoD**：有量化的延迟数据。

---

## 阶段 P2：系统级输入注入（2-3 天）

**目标**：替换 `dispatchGesture()` 为 `injectInputEvent()` 或 `input` 命令。

### 任务 P2.1：Shell 输入注入

| 文件 | 操作 | 说明 |
|------|------|------|
| `app/.../services/system/ShellInputInjector.kt` | 新增 | 通过 `input` 命令注入 |

支持操作：
- `input tap x y`
- `input swipe x1 y1 x2 y2 duration`
- `input keyevent KEYCODE_*`
- `input text "..."`

**DoD**：通过 Shizuku 调用 `input tap` 能在模拟器上点击。

### 任务 P2.2：InputManager 直接注入（Shizuku 专属）

| 文件 | 操作 | 说明 |
|------|------|------|
| `app/.../services/system/SystemInputInjector.kt` | 新增 | 通过反射调用 `InputManager.injectInputEvent()` |

实现方法：
- `tap(x, y)` — ACTION_DOWN + ACTION_UP
- `longPress(x, y, durationMs)` — ACTION_DOWN + 延迟 + ACTION_UP
- `doubleTap(x, y)` — 两次 tap 间隔 100ms
- `swipe(x1, y1, x2, y2, durationMs)` — DOWN + MOVE 序列 + UP
- `keyEvent(keyCode)` — ACTION_DOWN + ACTION_UP

**DoD**：`injectInputEvent` 调用成功，tap 延迟 <5ms。反射失败时降级到 `ShellInputInjector`。

### 任务 P2.3：App 管理

| 文件 | 操作 | 说明 |
|------|------|------|
| `app/.../services/system/AppController.kt` | 新增 | App 启动/停止/状态查询 |

```kotlin
interface AppController {
    fun launch(packageName: String): Result<Unit>
    fun forceStop(packageName: String): Result<Unit>
    fun getTopActivity(): String?
}
```

**DoD**：能通过 `am start` 启动设置 App，通过 `am force-stop` 停止。

---

## 阶段 P3：ToolRouter + 工具改造（3-4 天）

**目标**：所有现有 MCP 工具自动使用最快的可用路径，新增 3 个工具。

### 任务 P3.1：ToolRouter

| 文件 | 操作 | 说明 |
|------|------|------|
| `app/.../services/system/ToolRouter.kt` | 新增 | 统一路由层（见 DESIGN-V2-HEADLESS.md §3.5） |
| `app/.../di/AppModule.kt` | 修改 | 注册 ToolRouter 及其依赖 |

路由逻辑：

```
Shizuku 可用 → SystemScreenCapture + SystemInputInjector（最快）
       ↓ 不可用
Shell 可用 → ShellScreenCapture + ShellInputInjector（较快）
       ↓ 不可用
Accessibility → v1 路径（兜底）
```

**DoD**：ToolRouter 能正确检测当前可用模式并路由调用。

### 任务 P3.2：改造现有工具

| 文件 | 操作 | 变更内容 |
|------|------|---------|
| `ScreenIntrospectionTools.kt` | 修改 | 截帧部分改用 `ToolRouter.captureScreen()` |
| `TouchActionTools.kt` | 修改 | tap/swipe/longPress/doubleTap 改用 `ToolRouter.tap()` 等 |
| `SystemActionTools.kt` | 修改 | pressBack/pressHome 改用 `ToolRouter.keyEvent()` |
| `NodeActionTools.kt` | 不变 | 节点操作仍需 Accessibility API |
| `TextInputTools.kt` | 修改 | `type_text` 在 v2 模式下用 `input text` 命令 |
| `UtilityTools.kt` | 不变 | wait 逻辑不涉及输入/截帧 |

改造原则：
- 工具接口（参数、返回值）**完全不变**
- 只替换内部调用路径
- 降级到 v1 时行为与之前完全一致

**DoD**：所有 17 个工具在 Shizuku 模式和 Accessibility 模式下均通过测试。

### 任务 P3.3：新增 App 管理工具

| 文件 | 操作 | 说明 |
|------|------|------|
| `app/.../mcp/tools/AppTools.kt` | 新增 | 3 个工具 |
| `app/.../mcp/McpServer.kt` | 修改 | 注册新工具 |

新增工具：

| 工具 | 参数 | 返回 |
|------|------|------|
| `amctl_launch_app` | `package_name` (string) | `{ "success": true, "activity": "..." }` |
| `amctl_force_stop_app` | `package_name` (string) | `{ "success": true }` |
| `amctl_get_top_activity` | — | `{ "activity": "com.example/.MainActivity", "package": "com.example" }` |

**DoD**：`tools/list` 返回 20 个工具。新工具集成测试通过。

---

## 阶段 P4：犀牛派部署与验证（2-3 天）

**目标**：在犀牛派 X1 上跑通完整链路。

### 任务 P4.1：设备初始化

```bash
# 1. 连接犀牛派（USB 或 WiFi ADB）
adb connect <犀牛派IP>:5555

# 2. 安装 Shizuku
adb install shizuku-v13.1.5.apk

# 3. 启动 Shizuku 服务
adb shell sh /sdcard/Android/data/moe.shizuku.privileged.api/start.sh

# 4. 安装 amctl
adb install amctl-0.1.0-debug.apk

# 5. 启用无障碍服务（v1 fallback 保障）
adb shell settings put secure enabled_accessibility_services \
    com.example.amctl.debug/com.example.amctl.services.accessibility.AmctlAccessibilityService

# 6. 启动 MCP Server（通过 App UI 或 ADB）
adb shell am start -n com.example.amctl.debug/.ui.MainActivity
```

### 任务 P4.2：端到端测试

| 测试 | 操作 | 验证 |
|------|------|------|
| 截帧速度 | 调用 `amctl_get_screen_state` 100 次 | 平均延迟 <50ms (Shizuku) |
| 输入速度 | 调用 `amctl_tap` 100 次 | 平均延迟 <10ms (Shizuku) |
| App 管理 | `launch_app` → 截帧确认 → `force_stop` | 全流程成功 |
| 降级测试 | 停止 Shizuku → 调用工具 | 自动降级到 v1，功能正常 |
| 长时间稳定性 | 每 10 秒执行一次完整流程 | 运行 1 小时无异常 |

### 任务 P4.3：Shizuku 自动启动

犀牛派重启后 Shizuku 需要重新启动。解决方案：

```bash
# 方案一：ADB over WiFi 自连（推荐）
# 在 AidLux 的 Ubuntu 容器中设置 crontab
crontab -e
# 添加：
@reboot sleep 30 && adb connect localhost:5555 && \
    adb shell sh /sdcard/Android/data/moe.shizuku.privileged.api/start.sh

# 方案二：如果有 root
# 将 Shizuku 启动脚本放到 /data/adb/service.d/
echo '#!/system/bin/sh
sleep 10
sh /sdcard/Android/data/moe.shizuku.privileged.api/start.sh' \
    > /data/adb/service.d/start_shizuku.sh
chmod 755 /data/adb/service.d/start_shizuku.sh
```

**DoD**：设备重启后 Shizuku 自动恢复，amctl MCP Server 自动可用。

---

## 阶段 P5：justfile 更新 + 文档（1 天）

### 任务 P5.1：justfile 新增命令

| 文件 | 操作 | 说明 |
|------|------|------|
| `justfile` | 修改 | 新增犀牛派相关命令 |

```just
# 犀牛派部署
deploy-rhinopi ip="192.168.1.100":
    adb connect {{ ip }}:5555
    adb install -r app/build/outputs/apk/debug/amctl-0.1.0-debug.apk
    adb shell sh /sdcard/Android/data/moe.shizuku.privileged.api/start.sh

# Shizuku 状态检查
shizuku-status:
    adb shell dumpsys activity services | grep -i shizuku

# 性能基准测试
benchmark:
    @echo "截帧延迟测试 (10 次)..."
    @for i in $(seq 1 10); do \
        time curl -s -X POST http://localhost:8080/mcp \
            -H "Authorization: Bearer $(cat .token)" \
            -H "Content-Type: application/json" \
            -H "Accept: application/json, text/event-stream" \
            -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"amctl_get_screen_state","arguments":{}}}' > /dev/null; \
    done
```

### 任务 P5.2：更新文档

| 文件 | 操作 | 说明 |
|------|------|------|
| `README.md` | 修改 | 增加 v2 无头直控模式说明、Shizuku 配置步骤 |
| `docs/DESIGN.md` | 不变 | v1 文档保留 |

---

## 总体时间线

```
Day 1-3   ██████  P0: Shizuku 集成
Day 4-6   ██████  P1: 系统级截帧
Day 7-9   ██████  P2: 系统级输入注入
Day 10-13 ████████ P3: ToolRouter + 工具改造
Day 14-16 ██████  P4: 犀牛派部署与验证
Day 17    ██      P5: justfile + 文档
```

**总计：约 2.5 周**

---

## 执行注意事项

1. **先在模拟器上开发**，P0-P3 全部在模拟器上完成，P4 才上犀牛派
2. **不改工具接口**，只换内部实现，LLM prompt 零修改
3. **反射调用 hidden API 要做好降级**，每个反射调用都要 try-catch + fallback
4. **保留 v1 全部代码**，ToolRouter 降级时直接调用 v1 路径
5. **Shizuku 不是硬依赖**，没有 Shizuku 时 App 依然以 v1 模式完整可用
6. **测试覆盖**：每个工具都要测 Shizuku 模式 + Accessibility 模式两条路径
