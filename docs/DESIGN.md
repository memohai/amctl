# amctl — Android Mobile Control via MCP

## 项目概述

amctl 是一个 Android 应用，在设备上运行 MCP (Model Context Protocol) 服务器，让 AI 模型通过无障碍服务远程控制 Android 设备。

这是 [android-remote-control-mcp](https://github.com/niceguydave/android-remote-control-mcp) 的精简重写版，仅保留核心控制功能。

---

## 架构

### 三个核心组件

```
MCP 客户端 (AI)
    │
    ▼ HTTP POST /mcp + Bearer Token
┌────────────────────────────────────────────┐
│          McpServerService                   │
│          (Android 前台服务)                  │
│                                             │
│  Ktor HTTP Server (:8080)                   │
│    → BearerTokenAuth                        │
│    → StreamableHttp /mcp                    │
│    → MCP SDK Server                         │
│       → 注册的工具 (Tools)                   │
│          ├ ScreenIntrospectionTools          │
│          ├ TouchActionTools                  │
│          ├ NodeActionTools                   │
│          ├ TextInputTools                    │
│          ├ SystemActionTools                 │
│          └ UtilityTools                      │
└──────────────┬─────────────────────────────┘
               │ Singleton 引用
┌──────────────▼─────────────────────────────┐
│       McpAccessibilityService               │
│       (系统管理的无障碍服务)                  │
│                                             │
│  ├ AccessibilityTreeParser (解析 UI 树)      │
│  ├ ElementFinder (查找节点)                  │
│  ├ ActionExecutor (执行操作)                 │
│  └ ScreenCaptureProvider (截图)             │
└─────────────────────────────────────────────┘

┌─────────────────────────────────────────────┐
│       MainActivity (Compose UI)              │
│       配置 + 状态展示                         │
│                                              │
│  ├ MainViewModel (StateFlow)                 │
│  └ SettingsRepository → DataStore            │
└─────────────────────────────────────────────┘
```

### 服务间通信

- **McpAccessibilityService**: companion object 存储单例实例，MCP 工具直接引用
- **McpServerService**: companion-level StateFlow 暴露运行状态，UI 观察
- **Settings**: 通过 SettingsRepository 接口访问 DataStore

### 线程模型

| 线程 | 职责 |
|------|------|
| Main | Compose UI、Activity 生命周期、无障碍服务节点操作 |
| Dispatchers.IO | DataStore、Ktor 启动、网络 I/O |
| Dispatchers.Default | 截图编码、UI 树解析 |
| Ktor Netty 线程 | HTTP 请求处理 |

---

## 技术栈

| 技术 | 版本 | 用途 |
|------|------|------|
| Kotlin | 2.3.10 | 语言 |
| Android Gradle Plugin | 8.13 | 构建 |
| Gradle | 8.14.4 | 构建系统 |
| KSP | 2.3.5 | 注解处理 |
| Android SDK | Target 34, Min 33 | 平台 |
| JDK | 17 | 编译 |
| Jetpack Compose | BOM latest | UI |
| Material Design 3 | latest | UI 组件 |
| Hilt | latest | 依赖注入 |
| Ktor Server (Netty) | latest | HTTP 服务器 |
| MCP Kotlin SDK | 0.8.3 | MCP 协议 |
| Kotlinx Serialization | latest | JSON |
| Kotlinx Coroutines | latest | 异步 |
| DataStore Preferences | latest | 设置持久化 |
| SLF4J-Android | latest | MCP SDK 日志桥接 |

### 测试

| 技术 | 用途 |
|------|------|
| JUnit 5 | 测试框架 |
| MockK | Mocking |
| Turbine | Flow 测试 |
| Ktor Test Host | 集成测试 |
| MCP Kotlin SDK Client | MCP 集成测试 |

### 代码质量

| 工具 | 用途 |
|------|------|
| ktlint | Kotlin 格式化 |
| detekt | 静态分析 |

---

## 目录结构

```
amctl/
├── app/
│   ├── src/main/
│   │   ├── kotlin/com/example/amctl/
│   │   │   ├── AmctlApplication.kt
│   │   │   ├── di/
│   │   │   │   └── AppModule.kt
│   │   │   ├── data/
│   │   │   │   ├── model/
│   │   │   │   │   ├── ServerConfig.kt
│   │   │   │   │   ├── ServerStatus.kt
│   │   │   │   │   └── BindingAddress.kt
│   │   │   │   └── repository/
│   │   │   │       ├── SettingsRepository.kt
│   │   │   │       └── SettingsRepositoryImpl.kt
│   │   │   ├── services/
│   │   │   │   ├── accessibility/
│   │   │   │   │   ├── AmctlAccessibilityService.kt
│   │   │   │   │   ├── AccessibilityTreeParser.kt
│   │   │   │   │   ├── ElementFinder.kt
│   │   │   │   │   ├── ActionExecutor.kt
│   │   │   │   │   ├── ActionExecutorImpl.kt
│   │   │   │   │   ├── AccessibilityServiceProvider.kt
│   │   │   │   │   ├── AccessibilityServiceProviderImpl.kt
│   │   │   │   │   └── CompactTreeFormatter.kt
│   │   │   │   ├── screencapture/
│   │   │   │   │   ├── ScreenCaptureProvider.kt
│   │   │   │   │   ├── ScreenCaptureProviderImpl.kt
│   │   │   │   │   └── ScreenshotEncoder.kt
│   │   │   │   └── mcp/
│   │   │   │       └── McpServerService.kt
│   │   │   ├── mcp/
│   │   │   │   ├── McpServer.kt
│   │   │   │   ├── McpToolException.kt
│   │   │   │   ├── auth/
│   │   │   │   │   └── BearerTokenAuth.kt
│   │   │   │   └── tools/
│   │   │   │       ├── ScreenIntrospectionTools.kt
│   │   │   │       ├── TouchActionTools.kt
│   │   │   │       ├── NodeActionTools.kt
│   │   │   │       ├── TextInputTools.kt
│   │   │   │       ├── SystemActionTools.kt
│   │   │   │       └── UtilityTools.kt
│   │   │   ├── ui/
│   │   │   │   ├── MainActivity.kt
│   │   │   │   ├── theme/
│   │   │   │   │   ├── Theme.kt
│   │   │   │   │   ├── Color.kt
│   │   │   │   │   └── Type.kt
│   │   │   │   ├── screens/
│   │   │   │   │   └── HomeScreen.kt
│   │   │   │   ├── components/
│   │   │   │   │   ├── ServerStatusCard.kt
│   │   │   │   │   └── ConfigurationSection.kt
│   │   │   │   └── viewmodels/
│   │   │   │       └── MainViewModel.kt
│   │   │   └── utils/
│   │   │       ├── NetworkUtils.kt
│   │   │       └── PermissionUtils.kt
│   │   ├── res/
│   │   │   ├── values/strings.xml
│   │   │   ├── values/themes.xml
│   │   │   └── xml/accessibility_service_config.xml
│   │   └── AndroidManifest.xml
│   └── src/test/kotlin/
│       └── ... (单元测试 + 集成测试)
├── gradle/libs.versions.toml
├── build.gradle.kts
├── app/build.gradle.kts
├── settings.gradle.kts
├── gradle.properties
├── Makefile
└── README.md
```

---

## MCP 工具规格

### 传输层

- **端点**: `POST /mcp` (Streamable HTTP, JSON-only, 无 SSE)
- **认证**: `Authorization: Bearer <token>` (全局)
- **Content-Type**: `application/json`
- **协议**: JSON-RPC 2.0

### 工具列表 (17 个工具)

#### 1. Screen Introspection (1 工具)

| 工具 | 描述 | 参数 |
|------|------|------|
| `amctl_get_screen_state` | 获取屏幕状态：应用信息、屏幕尺寸、紧凑 TSV 格式 UI 节点列表 | `include_screenshot` (bool, 可选, 默认 false) |

输出格式：多窗口紧凑 TSV，过滤掉纯结构节点。文字/描述截断到 100 字符。
截图可选：低分辨率 JPEG (最大 700px)。

#### 2. Touch Actions (5 工具)

| 工具 | 描述 | 必填参数 | 可选参数 |
|------|------|---------|---------|
| `amctl_tap` | 单击坐标 | `x`, `y` | — |
| `amctl_long_press` | 长按坐标 | `x`, `y` | `duration` (ms, 默认 1000) |
| `amctl_double_tap` | 双击坐标 | `x`, `y` | — |
| `amctl_swipe` | 滑动 | `x1`, `y1`, `x2`, `y2` | `duration` (ms, 默认 300) |
| `amctl_scroll` | 方向滚动 | `direction` (up/down/left/right) | `amount` (small/medium/large) |

#### 3. Node Actions (4 工具)

| 工具 | 描述 | 必填参数 | 可选参数 |
|------|------|---------|---------|
| `amctl_find_nodes` | 查找 UI 节点 | `by` (text/content_desc/resource_id/class_name), `value` | `exact_match` (bool) |
| `amctl_click_node` | 点击节点 | `node_id` | — |
| `amctl_long_click_node` | 长按节点 | `node_id` | — |
| `amctl_scroll_to_node` | 滚动到节点 | `node_id` | — |

#### 4. Text Input (3 工具)

| 工具 | 描述 | 必填参数 | 可选参数 |
|------|------|---------|---------|
| `amctl_type_text` | 在文本框末尾输入文字 | `node_id`, `text` | — |
| `amctl_clear_text` | 清空文本框 | `node_id` | — |
| `amctl_press_key` | 按键 | `key` (ENTER/BACK/DEL/HOME/TAB/SPACE) | — |

#### 5. System Actions (2 工具)

| 工具 | 描述 | 参数 |
|------|------|------|
| `amctl_press_back` | 按返回键 | 无 |
| `amctl_press_home` | 按 Home 键 | 无 |

#### 6. Utility (2 工具)

| 工具 | 描述 | 必填参数 |
|------|------|---------|
| `amctl_wait_for_node` | 等待节点出现 | `by`, `value`, `timeout` (ms) |
| `amctl_wait_for_idle` | 等待 UI 空闲 | `timeout` (ms) |

### 错误处理

工具错误以 `CallToolResult(isError = true)` + TextContent 返回。
协议级错误由 MCP SDK 自动处理。

自定义异常类型：
- `McpToolException.PermissionDenied` — 无障碍未启用
- `McpToolException.InvalidParams` — 参数校验失败
- `McpToolException.ActionFailed` — 操作执行失败
- `McpToolException.NodeNotFound` — 节点未找到

---

## 数据模型

### ServerConfig

```kotlin
data class ServerConfig(
    val port: Int = 8080,
    val bindingAddress: BindingAddress = BindingAddress.LOCALHOST,
    val bearerToken: String = "",  // 首次读取时自动生成 UUID
    val autoStartOnBoot: Boolean = false,
)
```

### BindingAddress

```kotlin
enum class BindingAddress(val address: String) {
    LOCALHOST("127.0.0.1"),
    ALL_INTERFACES("0.0.0.0"),
}
```

### ServerStatus

```kotlin
sealed class ServerStatus {
    data object Stopped : ServerStatus()
    data object Starting : ServerStatus()
    data class Running(val port: Int, val address: String) : ServerStatus()
    data object Stopping : ServerStatus()
    data class Error(val message: String) : ServerStatus()
}
```

---

## UI 设计

单页面应用，HomeScreen 包含：

1. **ServerStatusCard** — 服务器状态（运行/停止）+ 启停按钮
2. **ConfigurationSection** — 端口、绑定地址(localhost/所有接口)、Bearer Token(显示/复制/重新生成)
3. **PermissionsSection** — 无障碍服务启用状态 + 跳转设置的按钮
4. **ConnectionInfoCard** — 设备 IP、端口、Token 信息

Material Design 3，支持 dark mode，最小触摸目标 48dp。
服务器运行时禁用配置编辑。

---

## 安全

- **Bearer Token**: 首次启动自动生成 UUID，DataStore 持久化，常量时间比较
- **网络绑定**: 默认 `127.0.0.1`，切换 `0.0.0.0` 时显示安全警告
- **无 HTTPS**: 精简版不实现 HTTPS，通过 adb 端口转发或局域网使用
- **权限最小化**: 仅 INTERNET、FOREGROUND_SERVICE、RECEIVE_BOOT_COMPLETED + 无障碍服务

---

## 默认配置

| 配置项 | 默认值 |
|--------|--------|
| Port | 8080 |
| Binding Address | 127.0.0.1 |
| Bearer Token | 自动生成 UUID |
| Auto-start on Boot | false |
| Screenshot Quality | 80 (JPEG) |
| Long Press Duration | 1000ms |
| Swipe Duration | 300ms |
| Scroll Amount | medium (50%) |

---

## 与参考项目的差异

| 特性 | android-remote-control-mcp | amctl |
|------|---------------------------|-------|
| MCP 工具数 | 54 | 17 |
| 文件操作 | 有 (8 工具 + SAF) | 无 |
| 相机工具 | 有 (6 工具) | 无 |
| 通知工具 | 有 (6 工具) | 无 |
| Intent 工具 | 有 (2 工具) | 无 |
| 应用管理 | 有 (3 工具) | 无 |
| 手势工具 | 有 (pinch, custom) | 无 |
| HTTPS | 可选 | 无 |
| 远程隧道 | Cloudflare + ngrok | 无 |
| 剪贴板 | 有 | 无 |
| 设备日志 | 有 | 无 |
| ADB 配置广播 | 有 | 无 |
| 截图标注 | 有 (红框 + ID 标签) | 无 |
| E2E 测试 | 有 (Testcontainers) | 无 |
| Kotlin 源文件数 | ~104 | ~30 |
