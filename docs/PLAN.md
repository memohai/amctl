# amctl — 实施计划

本文档是给 Cursor AI Agent 执行的逐步实施计划。
请先阅读 [DESIGN.md](DESIGN.md) 了解架构和技术选型。

参考项目源码位于：`/home/momo/dev/opensource/android-remote-control-mcp/`
可以参考其实现思路，但不要直接复制——amctl 是精简重写版。

---

## 阶段一：项目骨架

**目标**：创建 Android 项目，配置 Gradle，实现 Hilt 注入，跑通空白应用。

### 任务 1.1：初始化项目结构

创建新项目目录（位置由用户指定），生成以下文件：

| 文件 | 操作 | 说明 |
|------|------|------|
| `settings.gradle.kts` | 创建 | 项目名 "amctl"，include ":app" |
| `build.gradle.kts` (根) | 创建 | 声明插件（AGP 8.13, Kotlin 2.3.10, Hilt, KSP, Serialization, ktlint, detekt），不 apply |
| `app/build.gradle.kts` | 创建 | apply 插件，配置 SDK (compileSdk 36, minSdk 33, targetSdk 34)，声明所有依赖 |
| `gradle/libs.versions.toml` | 创建 | 版本目录，集中管理所有依赖版本 |
| `gradle.properties` | 创建 | `android.useAndroidX=true`, `android.nonTransitiveRClass=true`, VERSION_NAME=0.1.0, VERSION_CODE=1 |
| `gradle/wrapper/gradle-wrapper.properties` | 创建 | Gradle 8.14.4 wrapper |
| `.gitignore` | 创建 | Android 标准 gitignore |

**依赖列表**（app/build.gradle.kts）：
- AndroidX: core-ktx, activity-compose, lifecycle-viewmodel-compose, lifecycle-runtime-compose, datastore-preferences
- Compose: BOM, ui, material3, tooling-preview, tooling (debug)
- Hilt: hilt-android, hilt-compiler (ksp)
- Ktor Server: ktor-server-netty, ktor-server-content-negotiation, ktor-serialization-kotlinx-json
- MCP: mcp-kotlin-sdk (server)
- SLF4J: slf4j-android
- Kotlinx: serialization-json, coroutines-android
- 测试: junit-jupiter, mockk, turbine, ktor-server-test-host, mcp-kotlin-sdk (client)

**DoD**: `./gradlew assembleDebug` 成功（即使 app 是空的）。

### 任务 1.2：Application + Hilt + Manifest

| 文件 | 操作 | 说明 |
|------|------|------|
| `app/src/main/kotlin/com/example/amctl/AmctlApplication.kt` | 创建 | `@HiltAndroidApp` Application 类 |
| `app/src/main/AndroidManifest.xml` | 创建 | 声明 Application, MainActivity, McpServerService, AmctlAccessibilityService。权限: INTERNET, FOREGROUND_SERVICE, FOREGROUND_SERVICE_SPECIAL_USE, RECEIVE_BOOT_COMPLETED, POST_NOTIFICATIONS |
| `app/src/main/res/xml/accessibility_service_config.xml` | 创建 | 无障碍配置：所有事件类型，所有包名，FLAG_REPORT_VIEW_IDS + FLAG_RETRIEVE_INTERACTIVE_WINDOWS + FLAG_INPUT_METHOD_EDITOR |
| `app/src/main/res/values/strings.xml` | 创建 | 应用名 "amctl"，无障碍服务描述 |
| `app/src/main/res/values/themes.xml` | 创建 | Material3 主题 |

**DoD**: `./gradlew assembleDebug` 成功。

### 任务 1.3：数据层

| 文件 | 操作 | 说明 |
|------|------|------|
| `app/src/main/kotlin/.../data/model/ServerConfig.kt` | 创建 | 数据类，字段见 DESIGN.md |
| `app/src/main/kotlin/.../data/model/ServerStatus.kt` | 创建 | sealed class，见 DESIGN.md |
| `app/src/main/kotlin/.../data/model/BindingAddress.kt` | 创建 | enum，见 DESIGN.md |
| `app/src/main/kotlin/.../data/repository/SettingsRepository.kt` | 创建 | 接口：getServerConfig(), serverConfigFlow, updatePort(), updateBindingAddress(), updateBearerToken(), generateNewBearerToken(), updateAutoStartOnBoot() |
| `app/src/main/kotlin/.../data/repository/SettingsRepositoryImpl.kt` | 创建 | DataStore 实现。首次读取时自动生成 UUID bearer token |
| `app/src/main/kotlin/.../di/AppModule.kt` | 创建 | Hilt @Module：提供 DataStore 实例，@Binds SettingsRepository |

参考：`android-remote-control-mcp/app/src/main/kotlin/.../data/` 下的对应文件，但大幅简化（去掉 HTTPS、隧道、存储位置、文件大小限制等配置）。

**DoD**: 编译通过，数据层单元测试通过。

---

## 阶段二：无障碍服务

**目标**：实现 AccessibilityService，能解析 UI 树、查找节点、执行操作。

### 任务 2.1：核心无障碍服务

| 文件 | 操作 | 说明 |
|------|------|------|
| `app/src/main/kotlin/.../services/accessibility/AmctlAccessibilityService.kt` | 创建 | 继承 AccessibilityService。companion object 存储 @Volatile instance。onServiceConnected 设置实例，onDestroy 清除。onAccessibilityEvent 处理窗口变化事件 |
| `app/src/main/kotlin/.../services/accessibility/AccessibilityServiceProvider.kt` | 创建 | 接口：isServiceEnabled(), getRootNode(), getAccessibilityWindows(), performGlobalAction(), dispatchGesture(), takeScreenshot() |
| `app/src/main/kotlin/.../services/accessibility/AccessibilityServiceProviderImpl.kt` | 创建 | 实现，通过 AmctlAccessibilityService.instance 代理调用 |

参考：`android-remote-control-mcp/.../services/accessibility/` 下的对应文件。

**DoD**: 编译通过，服务能在模拟器上启用。

### 任务 2.2：UI 树解析

| 文件 | 操作 | 说明 |
|------|------|------|
| `app/src/main/kotlin/.../services/accessibility/AccessibilityTreeParser.kt` | 创建 | 接口 + 实现。parseTree(): 递归遍历 AccessibilityNodeInfo 树，生成 AccessibilityNodeData 列表。节点 ID 使用确定性哈希。支持多窗口 (getWindows()) 和降级模式 (rootInActiveWindow) |
| `app/src/main/kotlin/.../services/accessibility/CompactTreeFormatter.kt` | 创建 | 将解析结果格式化为紧凑 TSV：过滤纯结构节点，截断文字到 100 字符，使用缩写 flag |
| `app/src/main/kotlin/.../services/accessibility/ElementFinder.kt` | 创建 | 接口 + 实现。findByText/ContentDesc/ResourceId/ClassName，支持精确和模糊匹配 |

参考：`android-remote-control-mcp/.../services/accessibility/AccessibilityTreeParser.kt`、`CompactTreeFormatter.kt`、`ElementFinder.kt`。这些文件的核心逻辑直接复用，因为是基础设施。

**DoD**: 解析和查找的单元测试通过。

### 任务 2.3：操作执行器

| 文件 | 操作 | 说明 |
|------|------|------|
| `app/src/main/kotlin/.../services/accessibility/ActionExecutor.kt` | 创建 | 接口：tap(), longPress(), doubleTap(), swipe(), scroll(), performNodeAction(), performGlobalAction() |
| `app/src/main/kotlin/.../services/accessibility/ActionExecutorImpl.kt` | 创建 | 实现。使用 dispatchGesture() 执行触摸操作，performAction() 执行节点操作，performGlobalAction() 执行系统操作 |

参考：`android-remote-control-mcp/.../services/accessibility/ActionExecutor.kt` 和 `ActionExecutorImpl.kt`。

**DoD**: 编译通过，操作执行器单元测试通过。

### 任务 2.4：截图

| 文件 | 操作 | 说明 |
|------|------|------|
| `app/src/main/kotlin/.../services/screencapture/ScreenCaptureProvider.kt` | 创建 | 接口：captureScreenshot(): ScreenshotData? |
| `app/src/main/kotlin/.../services/screencapture/ScreenCaptureProviderImpl.kt` | 创建 | 通过 AccessibilityService.takeScreenshot() API (Android 11+) 实现 |
| `app/src/main/kotlin/.../services/screencapture/ScreenshotEncoder.kt` | 创建 | Bitmap → JPEG base64 编码，支持缩放到最大尺寸 |

参考：`android-remote-control-mcp/.../services/screencapture/` 下的对应文件。不需要 ScreenshotAnnotator（截图标注），精简版不做标注。

**DoD**: 编译通过，截图编码器单元测试通过。

---

## 阶段三：MCP 服务器 + 工具

**目标**：实现 Ktor HTTP 服务器、MCP 协议集成、所有 17 个 MCP 工具。

### 任务 3.1：MCP 服务器基础

| 文件 | 操作 | 说明 |
|------|------|------|
| `app/src/main/kotlin/.../mcp/McpToolException.kt` | 创建 | sealed class：PermissionDenied, InvalidParams, ActionFailed, NodeNotFound |
| `app/src/main/kotlin/.../mcp/auth/BearerTokenAuth.kt` | 创建 | Ktor Application-level plugin。提取 Authorization header，常量时间比较 token，无效返回 401。/health 端点不验证 |
| `app/src/main/kotlin/.../mcp/McpServer.kt` | 创建 | 创建 Ktor 嵌入式服务器 (Netty)，安装 BearerTokenAuth，配置 /mcp 路由 (StreamableHttpServerTransport, JSON-only)，注册 /health 端点，注册所有 MCP 工具 |

参考：
- `android-remote-control-mcp/.../mcp/McpServer.kt` — 核心结构
- `android-remote-control-mcp/.../mcp/auth/BearerTokenAuth.kt` — 认证插件
- `android-remote-control-mcp/.../mcp/McpStreamableHttpExtension.kt` — Streamable HTTP 路由

**DoD**: 编译通过，BearerTokenAuth 单元测试通过。

### 任务 3.2：McpServerService（前台服务）

| 文件 | 操作 | 说明 |
|------|------|------|
| `app/src/main/kotlin/.../services/mcp/McpServerService.kt` | 创建 | Android 前台服务。onStartCommand: startForeground() + 读取 ServerConfig + 创建 McpServer + 启动 Ktor。onDestroy: 优雅停止 Ktor + 取消协程。companion object: StateFlow<ServerStatus>, ACTION_START/STOP 常量 |

参考：`android-remote-control-mcp/.../services/mcp/McpServerService.kt`，去掉隧道、HTTPS、日志事件等。

**DoD**: 服务能启动并响应 /health 请求。

### 任务 3.3：Screen Introspection Tool

| 文件 | 操作 | 说明 |
|------|------|------|
| `app/src/main/kotlin/.../mcp/tools/ScreenIntrospectionTools.kt` | 创建 | `amctl_get_screen_state`：调用 TreeParser 获取多窗口 UI 树，通过 CompactTreeFormatter 格式化为 TSV，可选包含截图 |

参考：`android-remote-control-mcp/.../mcp/tools/ScreenIntrospectionTools.kt`。去掉截图标注功能。

**DoD**: 通过 curl 调用工具返回 UI 树 TSV。集成测试通过。

### 任务 3.4：Touch Action Tools

| 文件 | 操作 | 说明 |
|------|------|------|
| `app/src/main/kotlin/.../mcp/tools/TouchActionTools.kt` | 创建 | 5 个工具：tap, long_press, double_tap, swipe, scroll。参数校验 + 调用 ActionExecutor |

参考：`android-remote-control-mcp/.../mcp/tools/TouchActionTools.kt`。

**DoD**: 集成测试通过。

### 任务 3.5：Node Action Tools

| 文件 | 操作 | 说明 |
|------|------|------|
| `app/src/main/kotlin/.../mcp/tools/NodeActionTools.kt` | 创建 | 4 个工具：find_nodes, click_node, long_click_node, scroll_to_node。调用 ElementFinder 和 ActionExecutor |

参考：`android-remote-control-mcp/.../mcp/tools/NodeActionTools.kt`。

**DoD**: 集成测试通过。

### 任务 3.6：Text Input Tools

| 文件 | 操作 | 说明 |
|------|------|------|
| `app/src/main/kotlin/.../mcp/tools/TextInputTools.kt` | 创建 | 3 个工具：type_text (末尾追加), clear_text (select-all + delete), press_key (ENTER/BACK/DEL/HOME/TAB/SPACE) |

参考：`android-remote-control-mcp/.../mcp/tools/TextInputTools.kt`。精简版只保留 type_append 和 clear，不需要 insert/replace。type_text 使用 InputConnection commitText API 逐字符输入。

关于 TypeInputController：
- 如果需要自然输入（逐字符 + 延迟），需要从参考项目复制 `TypeInputController.kt` 和 `TypeInputControllerImpl.kt`
- 精简方案：直接用 `AccessibilityNodeInfo.performAction(ACTION_SET_TEXT)` 整体设置文字，更简单但不够自然

建议先用 `ACTION_SET_TEXT` 简单方案，后续再升级为 InputConnection 逐字符输入。

**DoD**: 集成测试通过。

### 任务 3.7：System Action + Utility Tools

| 文件 | 操作 | 说明 |
|------|------|------|
| `app/src/main/kotlin/.../mcp/tools/SystemActionTools.kt` | 创建 | 2 个工具：press_back (GLOBAL_ACTION_BACK), press_home (GLOBAL_ACTION_HOME) |
| `app/src/main/kotlin/.../mcp/tools/UtilityTools.kt` | 创建 | 2 个工具：wait_for_node (轮询 500ms 直到节点出现), wait_for_idle (比较 UI 树指纹直到稳定) |

参考：
- `android-remote-control-mcp/.../mcp/tools/SystemActionTools.kt`
- `android-remote-control-mcp/.../mcp/tools/UtilityTools.kt`

wait_for_idle 需要 TreeFingerprint 支持。可以从参考项目复制 `TreeFingerprint.kt`，或者用简单的树哈希比较。

**DoD**: 集成测试通过。

### 任务 3.8：工具注册

在 `McpServer.kt` 中使用 `Server.addTool()` 注册所有 17 个工具。每个工具类提供一个 `register(server: Server)` 方法。

参考：`android-remote-control-mcp/.../mcp/McpServer.kt` 中的工具注册方式。

**DoD**: `tools/list` 返回 17 个工具。

---

## 阶段四：UI

**目标**：实现配置界面，能启停服务器、修改配置、查看状态。

### 任务 4.1：ViewModel

| 文件 | 操作 | 说明 |
|------|------|------|
| `app/src/main/kotlin/.../ui/viewmodels/MainViewModel.kt` | 创建 | @HiltViewModel。暴露 serverStatus (StateFlow), serverConfig (StateFlow)。方法：startServer(), stopServer(), updatePort(), updateBindingAddress(), updateBearerToken(), generateNewBearerToken() |

参考：`android-remote-control-mcp/.../ui/viewmodels/MainViewModel.kt`，大幅简化。

**DoD**: ViewModel 单元测试通过。

### 任务 4.2：Theme

| 文件 | 操作 | 说明 |
|------|------|------|
| `app/src/main/kotlin/.../ui/theme/Color.kt` | 创建 | Material3 颜色定义 |
| `app/src/main/kotlin/.../ui/theme/Type.kt` | 创建 | Material3 字体定义 |
| `app/src/main/kotlin/.../ui/theme/Theme.kt` | 创建 | 亮色/暗色主题，跟随系统 |

可以从参考项目直接复制这三个文件。

### 任务 4.3：UI 组件

| 文件 | 操作 | 说明 |
|------|------|------|
| `app/src/main/kotlin/.../ui/components/ServerStatusCard.kt` | 创建 | 显示服务器状态 (Stopped/Starting/Running/Error) + 启停按钮 (Switch 或 Button) |
| `app/src/main/kotlin/.../ui/components/ConfigurationSection.kt` | 创建 | 端口 (OutlinedTextField)、绑定地址 (RadioButton: localhost/所有接口)、Bearer Token (显示/复制/重新生成)。服务器运行时禁用编辑 |

参考：`android-remote-control-mcp/.../ui/components/` 下的对应文件，但只保留核心配置。

### 任务 4.4：HomeScreen + MainActivity

| 文件 | 操作 | 说明 |
|------|------|------|
| `app/src/main/kotlin/.../ui/screens/HomeScreen.kt` | 创建 | TopAppBar + 滚动布局：ServerStatusCard → ConfigurationSection → 无障碍权限状态 + 跳转按钮 → 连接信息 (IP + 端口 + Token) |
| `app/src/main/kotlin/.../ui/MainActivity.kt` | 创建 | @AndroidEntryPoint，setContent { AmctlTheme { HomeScreen() } } |

**DoD**: 应用能安装到模拟器，UI 能正常显示，能通过 UI 启停服务器。

---

## 阶段五：justfile + 工具

**目标**：开发工具链。

### 任务 5.1：justfile

| 文件 | 操作 | 说明 |
|------|------|------|
| `justfile` | 创建 | recipes：build, clean, install, uninstall, test, lint, lint-fix, emulator-headless, emulator-ui, emulator-stop, setup-emulator, forward-port, grant-permissions, start, logs, check, deploy, all |

参考：`android-remote-control-mcp/Makefile`，去掉 E2E 测试、cloudflared 编译、ngrok 编译、版本号管理等。使用 just 替代 make。

**DoD**: `just build`, `just install`, `just lint` 正常工作。

---

## 阶段六：测试 + 质量

**目标**：单元测试和集成测试覆盖核心逻辑。

### 任务 6.1：单元测试

为以下模块编写单元测试：

| 测试文件 | 覆盖内容 |
|----------|---------|
| `SettingsRepositoryImplTest.kt` | 配置读写、Token 自动生成 |
| `BearerTokenAuthTest.kt` | 认证成功/失败、常量时间比较 |
| `AccessibilityTreeParserTest.kt` | 树解析、节点 ID 生成 |
| `ElementFinderTest.kt` | 查找匹配 |
| `CompactTreeFormatterTest.kt` | TSV 格式化、节点过滤 |
| `ActionExecutorImplTest.kt` | 操作执行 |
| `ScreenshotEncoderTest.kt` | 编码、缩放 |
| `TouchActionToolsTest.kt` | 参数校验、工具执行 |
| `NodeActionToolsTest.kt` | 参数校验、节点查找 |
| `TextInputToolsTest.kt` | 参数校验、文字操作 |
| `SystemActionToolsTest.kt` | 全局操作 |
| `UtilityToolsTest.kt` | wait_for_node, wait_for_idle |
| `MainViewModelTest.kt` | 状态管理 |
| `ServerConfigTest.kt` | 默认值 |

### 任务 6.2：集成测试

| 测试文件 | 覆盖内容 |
|----------|---------|
| `McpIntegrationTestHelper.kt` | 测试基础设施：配置 Ktor testApplication + MCP SDK Client |
| `AuthIntegrationTest.kt` | Token 认证 |
| `TouchActionIntegrationTest.kt` | 触摸工具 HTTP 全栈 |
| `NodeActionIntegrationTest.kt` | 节点工具 HTTP 全栈 |
| `TextInputIntegrationTest.kt` | 文字工具 HTTP 全栈 |
| `SystemActionIntegrationTest.kt` | 系统操作 HTTP 全栈 |
| `ScreenIntrospectionIntegrationTest.kt` | 屏幕状态 HTTP 全栈 |
| `UtilityIntegrationTest.kt` | 工具类 HTTP 全栈 |

集成测试使用 Ktor `testApplication`（JVM 层，不需要模拟器），Mock Android 服务接口。

参考：`android-remote-control-mcp/app/src/test/kotlin/.../integration/` 下的对应文件和 `McpIntegrationTestHelper.kt`。

**DoD**: `make test-unit` 全部通过，`make lint` 无错误。

---

## 执行注意事项

1. **按阶段顺序执行**，每个阶段内的任务也按顺序执行
2. **每个任务完成后验证 DoD**，确保编译/测试通过再继续
3. **大量参考项目源码**在 `/home/momo/dev/opensource/android-remote-control-mcp/`，尤其是：
   - 无障碍相关的代码（TreeParser, ElementFinder, ActionExecutor）逻辑复杂，建议仔细阅读理解后再简化实现
   - MCP SDK 的使用方式（Server.addTool, StreamableHttpServerTransport）直接参考
   - Ktor 路由配置和 BearerTokenAuth 插件可以复用核心逻辑
4. **包名**使用 `com.example.amctl`（用户可后续修改）
5. **不要添加 DESIGN.md 中未列出的功能**
6. **所有接口定义为 interface**，实现类后缀 Impl，通过 Hilt @Binds 绑定
7. **Kotlin 协程**：ViewModel 用 viewModelScope，Service 用自定义 CoroutineScope 并在 onDestroy 取消
8. **线程安全**：AccessibilityService.instance 用 @Volatile，节点操作必须在主线程
