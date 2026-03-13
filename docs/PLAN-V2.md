# amctl v2 — 定制 ROM 实施计划

本文档是 amctl v2（定制 ROM 方案）的逐步实施计划。
请先阅读 [DESIGN-V2-ROM.md](DESIGN-V2-ROM.md) 了解完整架构和技术选型。

v1 源码位于当前项目中，v2 在此基础上扩展，保持 v1 功能作为 fallback。

---

## 前提条件

在开始 v2 开发之前，确保：

1. **v1 功能完整可用** — 所有 17 个 MCP 工具通过测试
2. **犀牛派 X1 硬件到位** — QCS8550 ARM64 开发板
3. **编译服务器就绪** — 16 核 / 64GB 内存 / 500GB SSD（本地笔记本不足，建议云服务器）
4. **BSP 源码获取** — 已联系阿加犀（AidLux）获取犀牛派 X1 的 AOSP BSP
5. **官方 ROM 备份** — 刷机前备份完整官方 ROM（用于救砖）
6. **刷机工具** — Linux 上安装 `qdl`，或 Windows 上安装 QPST/QFIL

---

## 阶段 P0：AOSP 环境搭建（1-2 周）

**目标**：AOSP + QCS8550 BSP 编译通过，通过 QFIL/qdl 刷入犀牛派 X1 正常启动。

### 任务 P0.1：获取 BSP 源码

| 步骤 | 操作 | 说明 |
|------|------|------|
| 1 | 联系阿加犀获取 BSP | 邮件/工单请求犀牛派 X1 AOSP 源码包 |
| 2 | 备选：CodeLinaro/CAF | 如果厂商不提供，从 `git.codelinaro.org/clo/la/platform/manifest` 获取 QCS8550 公版 BSP |
| 3 | 确认 BSP 完整性 | 检查 `device/`、`kernel/`、`vendor/` 三个目录是否齐全 |

**如果阿加犀提供完整 AOSP 源码包**：直接使用，跳过步骤 2。

**如果只提供二进制 ROM + 部分 BSP**：

```bash
# 从官方 ROM 中提取 vendor blobs
# 1. 先刷入官方 ROM
# 2. 通过 adb 提取
adb root
adb pull /vendor/ ~/rhinopi-vendor/
adb pull /system/ ~/rhinopi-system/

# 3. 使用 extract-files.sh（如果 BSP 提供了的话）
cd device/qualcomm/rhinopi_x1/
./extract-files.sh ~/rhinopi-vendor/ ~/rhinopi-system/
```

**DoD**：拥有完整可编译的 BSP 源码。

### 任务 P0.2：搭建编译环境

| 步骤 | 操作 | 说明 |
|------|------|------|
| 1 | 准备编译机 | Ubuntu 22.04，16 核 / 64GB / 500GB SSD |
| 2 | 安装依赖 | AOSP 编译工具链（见下方） |
| 3 | 配置 repo | 安装 Google 的 `repo` 工具 |
| 4 | 配置 ccache | 加速增量编译 |

```bash
# Ubuntu 22.04 编译依赖
sudo apt-get install -y git-core gnupg flex bison build-essential \
    zip curl zlib1g-dev libc6-dev-i386 x11proto-core-dev \
    libx11-dev lib32z1-dev libgl1-mesa-dev libxml2-utils \
    xsltproc unzip fontconfig python3 python3-pip openjdk-17-jdk

# 安装 repo
mkdir -p ~/bin
curl https://storage.googleapis.com/git-repo-downloads/repo > ~/bin/repo
chmod a+x ~/bin/repo
export PATH=~/bin:$PATH

# 配置 ccache（推荐 50-100GB）
export USE_CCACHE=1
export CCACHE_DIR=~/.ccache
ccache -M 100G
```

**DoD**：编译环境就绪，`javac --version` 显示 OpenJDK 17。

### 任务 P0.3：同步 AOSP 源码

```bash
mkdir ~/aosp-rhinopi && cd ~/aosp-rhinopi

# 方式一：使用阿加犀提供的 manifest（优先）
repo init -u <阿加犀提供的 manifest URL> -b <分支名>
repo sync -j16 --no-tags

# 方式二：使用 CodeLinaro/CAF 公版 + BSP 合入
repo init -u https://git.codelinaro.org/clo/la/platform/manifest \
    -b release -m LA.QSSI.14.0.r1-xxxxx-qssi.0.xml
repo sync -j16 --no-tags

# 合入犀牛派 BSP
cp -r <bsp_path>/device/qualcomm/rhinopi_x1 device/qualcomm/rhinopi_x1
cp -r <bsp_path>/kernel/msm-5.15 kernel/msm-5.15
cp -r <bsp_path>/vendor/qcom vendor/qcom
```

**预期耗时**：repo sync 约 2-6 小时（取决于网络，源码约 100-200GB）。

**DoD**：`repo sync` 完成，无错误。

### 任务 P0.4：首次编译

```bash
cd ~/aosp-rhinopi
source build/envsetup.sh
lunch rhinopi_x1-userdebug
make -j$(nproc)
```

| 情况 | 处理方式 |
|------|---------|
| 编译成功 | 继续 P0.5 |
| vendor blob 缺失报错 | 从官方 ROM 提取（任务 P0.1 备选路径） |
| kernel 编译失败 | 检查 BSP 的 kernel config，确认高通 defconfig 正确 |
| SELinux 报错 | 暂时 `SELINUX=permissive`，后续 P6 再收紧 |

**预期耗时**：首次全量编译约 3-5 小时（16 核 / 64GB 服务器）。

**DoD**：编译产物在 `out/target/product/rhinopi_x1/` 下生成。

### 任务 P0.5：刷机与验证

#### 准备工作

```bash
# 备份官方 ROM（极其重要！）
# 如果阿加犀提供了官方 ROM 下载包，先保存一份
# 如果只有设备上的系统，通过 dd 备份关键分区

adb root
adb shell dd if=/dev/block/by-name/boot of=/sdcard/boot_backup.img
adb shell dd if=/dev/block/by-name/system of=/sdcard/system_backup.img
adb pull /sdcard/boot_backup.img ~/rhinopi-backup/
adb pull /sdcard/system_backup.img ~/rhinopi-backup/
```

#### 刷机

```bash
# Linux 推荐使用 qdl
# 安装 qdl
git clone https://github.com/linux-msm/qdl.git
cd qdl && make && sudo make install

# 设备进入 EDL 模式
adb reboot edl
# 等待 9008 端口出现：ls /dev/ttyUSB*

# 刷机
cd ~/aosp-rhinopi/out/target/product/rhinopi_x1/
sudo qdl prog_firehose_ddr.elf rawprogram*.xml patch*.xml
```

#### 验证

```bash
# 等待设备启动完成（首次启动可能需要 3-5 分钟）
adb wait-for-device

# 验证自定义 ROM
adb shell getprop ro.build.display.id       # 应显示自定义 build ID
adb shell getprop ro.build.type             # 应显示 "userdebug"
adb shell getprop ro.hardware               # 应显示 rhinopi 相关
adb shell cat /proc/cpuinfo | head -20      # 确认 ARM64 QCS8550
adb shell free -h                           # 确认 16GB 内存可用

# 功能验证
adb shell settings list system              # 基本系统服务正常
adb shell dumpsys display                   # Display 服务正常
adb shell dumpsys input                     # Input 服务正常
```

**DoD**：设备正常启动进入 Android 桌面，ADB 连接正常，上述验证命令全部通过。

---

## 阶段 P1：虚拟 Display（2-3 周）

**目标**：实现 `AmctlSystemService`，可创建虚拟 Display 并在上面启动 Activity。

### 任务 P1.1：AIDL 接口定义

| 文件 | 操作 | 说明 |
|------|------|------|
| `frameworks/base/core/java/android/amctl/IAmctlService.aidl` | 新增 | 完整 AIDL 接口（见 DESIGN-V2-ROM.md §3.5） |
| `frameworks/base/core/java/android/amctl/IFrameCallback.aidl` | 新增 | 帧回调接口 |
| `frameworks/base/Android.bp` | 修改 | 将 AIDL 文件加入编译 |

AIDL 接口包含 6 组方法：Display 管理（4）、App 管理（4）、输入注入（6）、帧捕获（3）、UI 树（2）、全局操作（3）。

**DoD**：`make framework-minus-apex` 编译通过，生成 AIDL 的 Java 桩代码。

### 任务 P1.2：AmctlSystemService 骨架

| 文件 | 操作 | 说明 |
|------|------|------|
| `frameworks/base/services/core/java/com/android/server/amctl/AmctlSystemService.java` | 新增 | 继承 `SystemService`，实现 `IAmctlService.Stub`，空方法 |
| `frameworks/base/services/java/com/android/server/SystemServer.java` | 修改 | 在 `startOtherServices()` 中注册 AmctlSystemService |

```java
// SystemServer.java 修改点
private void startOtherServices() {
    // ... 已有代码 ...

    // 注册 amctl 系统服务
    traceBeginAndSlog("StartAmctlService");
    mSystemServiceManager.startService(AmctlSystemService.class);
    traceEnd();

    // ... 已有代码继续 ...
}
```

**验证**：
```bash
make framework-minus-apex services -j$(nproc)
adb root && adb remount
adb push out/.../framework.jar /system/framework/
adb push out/.../services.jar /system/framework/
adb reboot

# 验证服务注册
adb shell service list | grep amctl    # 应出现 amctl_service
```

**DoD**：`service list` 中可见 `amctl_service`。

### 任务 P1.3：DisplayController — 虚拟 Display 创建

| 文件 | 操作 | 说明 |
|------|------|------|
| `frameworks/base/services/core/java/com/android/server/amctl/DisplayController.java` | 新增 | 虚拟 Display 的创建/销毁/配置 |
| `frameworks/base/services/core/java/com/android/server/amctl/DisplayPool.java` | 新增 | Display 池管理（LRU 淘汰，最大 8 个） |

核心逻辑：
- 使用 `DisplayManager.createVirtualDisplay()` 创建，flags 为 `PUBLIC | OWN_CONTENT_ONLY | SUPPORTS_TOUCH`
- 绑定 ImageReader 的 Surface（为 P3 帧捕获做准备）
- 维护 `SparseArray<DisplaySlot>` 管理活跃 Display

**DoD**：
```bash
# 通过 adb 调用 AIDL 创建虚拟 Display
adb shell service call amctl_service 1 i32 1080 i32 2400 i32 420
# 返回 displayId > 0

adb shell dumpsys display | grep "amctl-display"  # 可见虚拟 Display
```

### 任务 P1.4：AOSP 修改 — 跨 Display 启动 Activity

| 文件 | 操作 | 说明 |
|------|------|------|
| `frameworks/base/services/core/java/com/android/server/wm/ActivityStarter.java` | 修改 | 放行 amctl 管理的 Display 上的 Activity 启动 |
| `frameworks/base/services/core/java/com/android/server/display/DisplayManagerService.java` | 修改 | 允许系统服务创建触摸可用的 VirtualDisplay |

ActivityStarter 修改要点：
- 在 `isAllowedToStartOnDisplay()` 中增加判断：如果 `targetDisplayId` 属于 amctl 管理的虚拟 Display，直接放行
- 通过 `AmctlSystemService.isAmctlManagedDisplay(displayId)` 查询

**DoD**：
```bash
# 在虚拟 Display 上启动设置
adb shell service call amctl_service 5 i32 <displayId> s16 "com.android.settings"

# 验证 Activity 在虚拟 Display 上运行
adb shell dumpsys activity activities | grep "displayId=<displayId>"
```

### 任务 P1.5：SELinux 策略（初始版）

| 文件 | 操作 | 说明 |
|------|------|------|
| `device/qualcomm/rhinopi_x1/sepolicy/amctl_service.te` | 新增 | amctl 系统服务的 SELinux 策略 |
| `device/qualcomm/rhinopi_x1/sepolicy/file_contexts` | 修改 | 声明 amctl_service context |
| `device/qualcomm/rhinopi_x1/sepolicy/service_contexts` | 修改 | 注册 amctl_service binder context |

初期策略：
- P1 阶段先使用 `setenforce 0`（permissive）快速迭代
- 每次遇到 `avc denied` 时记录，P6 阶段统一收紧为 enforcing

**DoD**：虚拟 Display 创建和 Activity 启动在 permissive 模式下无 SELinux 相关 crash。

---

## 阶段 P2：输入注入（1-2 周）

**目标**：可向虚拟 Display 注入 tap/swipe/key/text，与物理屏幕完全隔离。

### 任务 P2.1：InputInjector 实现

| 文件 | 操作 | 说明 |
|------|------|------|
| `frameworks/base/services/core/java/com/android/server/amctl/InputInjector.java` | 新增 | 输入注入器（见 DESIGN-V2-ROM.md §3.2.2） |

实现方法：
- `injectTap(displayId, x, y)` — ACTION_DOWN + ACTION_UP，间隔 50ms
- `injectLongPress(displayId, x, y, durationMs)` — ACTION_DOWN + 延迟 + ACTION_UP
- `injectDoubleTap(displayId, x, y)` — 两次 tap，间隔 100ms
- `injectSwipe(displayId, x1, y1, x2, y2, durationMs)` — DOWN + MOVE 序列（16ms/步）+ UP
- `injectKeyEvent(displayId, keyCode)` — ACTION_DOWN + ACTION_UP
- `injectText(displayId, text)` — 通过 InputConnection.commitText()

所有 MotionEvent 必须设置正确的 `displayId`，通过 `event.setDisplayId(displayId)` 确保路由隔离。

**DoD**：
```bash
# 在虚拟 Display 上启动一个 App，然后注入 tap
adb shell service call amctl_service 10 i32 <displayId> f 540.0 f 1200.0
# 虚拟 Display 上的 App 响应点击，物理屏幕无任何变化
```

### 任务 P2.2：验证输入隔离

| 测试 | 方法 | 预期 |
|------|------|------|
| 物理屏不受影响 | 向 VD 注入 tap，同时观察物理屏 | 物理屏幕上的 App 无任何响应 |
| VD 正确响应 | 向 VD 上的按钮注入 tap | 按钮被点击 |
| 多 VD 隔离 | 创建 2 个 VD，分别注入 | 各 VD 独立响应 |
| Key 事件路由 | 向 VD 注入 BACK/HOME | 仅影响目标 VD |

### 任务 P2.3：InputDispatcher 验证

| 文件 | 操作 | 说明 |
|------|------|------|
| `frameworks/native/services/inputflinger/dispatcher/InputDispatcher.cpp` | 验证 | 确认 `injectInputEvent()` 在指定 displayId 时不 fallback 到 focused display |

如果发现路由异常（事件跑到物理屏），需要修改 InputDispatcher 的 `findFocusedWindowTargetsLocked()` 方法，确保按 displayId 查找目标窗口。

**DoD**：输入隔离测试全部通过。AIDL 中 6 个输入方法全部可用。

---

## 阶段 P3：帧捕获（1-2 周）

**目标**：可从虚拟 Display 捕获帧数据（JPEG），延迟 <10ms。

### 任务 P3.1：FrameCapture 实现

| 文件 | 操作 | 说明 |
|------|------|------|
| `frameworks/base/services/core/java/com/android/server/amctl/FrameCapture.java` | 新增 | 帧捕获模块（见 DESIGN-V2-ROM.md §3.3.2） |

核心流程：
1. 虚拟 Display 创建时（P1.3），绑定 `ImageReader` 的 Surface
2. `captureFrame(displayId)` → `ImageReader.acquireLatestImage()` → `HardwareBuffer` → `Bitmap.wrapHardwareBuffer()` → JPEG 编码

关键点：
- `ImageReader` 使用 `HardwareBuffer.USAGE_GPU_SAMPLED_IMAGE | USAGE_CPU_READ_OFTEN`
- buffer 数量设为 2（双缓冲，避免延迟）
- JPEG 编码在后台线程执行

**DoD**：
```bash
# 通过 AIDL 调用截图
adb shell service call amctl_service 20 i32 <displayId> i32 720 i32 1280 i32 80
# 返回 JPEG 数据（验证非空）

# 性能验证
adb shell "time service call amctl_service 20 ..."  # 延迟应 <10ms（不含传输）
```

### 任务 P3.2：帧流模式（可选，P6 可推迟）

| 文件 | 操作 | 说明 |
|------|------|------|
| `FrameCapture.java` | 扩展 | 实现 `startFrameStream()` / `stopFrameStream()`，通过 `IFrameCallback` 推送帧 |

帧流模式用于实时监控虚拟 Display，通过 `ImageReader.setOnImageAvailableListener()` 在每帧 ready 时回调。初期可跳过，P6 按需实现。

**DoD**：按需截图功能可用，JPEG 数据正确。

---

## 阶段 P4：UI 树读取（1 周）

**目标**：可读取虚拟 Display 上 App 的 UI 树，输出与 v1 兼容的 TSV 格式。

### 任务 P4.1：UiTreeReader 实现

| 文件 | 操作 | 说明 |
|------|------|------|
| `frameworks/base/services/core/java/com/android/server/amctl/UiTreeReader.java` | 新增 | UI 树读取器（见 DESIGN-V2-ROM.md §3.4） |

实现要点：
- 通过 `AccessibilityManagerService.getWindowsOnDisplay(displayId)` 获取指定 Display 的窗口列表
- 递归遍历 `AccessibilityNodeInfo` 树
- 输出格式与 v1 的 `CompactTreeFormatter` 完全一致（TSV：node_id, class, text, desc, res_id, bounds, flags）
- 支持 `findNodes(displayId, by, value, exactMatch)` 查询

**格式兼容性至关重要**：LLM 端的 prompt 不需要因为 v2 而修改。

**DoD**：
```bash
# 在虚拟 Display 上启动设置 App
adb shell service call amctl_service 5 i32 <displayId> s16 "com.android.settings"

# 读取 UI 树
adb shell service call amctl_service 30 i32 <displayId>
# 返回 TSV 格式的 UI 树，内容与物理屏 v1 输出格式一致

# 节点查找
adb shell service call amctl_service 31 i32 <displayId> s16 "text" s16 "搜索" i32 0
# 返回匹配节点
```

### 任务 P4.2：与 v1 输出对比验证

| 验证项 | 方法 |
|--------|------|
| TSV 列数一致 | 对比 v1 和 v2 输出的列数 |
| 节点 ID 格式一致 | 确认都使用确定性哈希 |
| 中文文本正确 | 验证 UTF-8 编码无乱码 |
| 多窗口支持 | 打开对话框，验证多窗口节点都被捕获 |

**DoD**：同一个 App 界面，v1（物理屏）和 v2（虚拟 Display）的 UI 树输出格式完全一致。

---

## 阶段 P5：AIDL + MCP 整合（1-2 周）

**目标**：系统服务通过 AIDL 暴露给 App 进程的 MCP Server，MCP 工具支持 `display_id` 参数。

### 任务 P5.1：AIDL 客户端封装

| 文件 | 操作 | 说明 |
|------|------|------|
| `app/.../services/system/AmctlServiceClient.kt` | 新增 | AIDL 客户端封装，绑定系统服务 |
| `app/.../services/system/AmctlServiceClientImpl.kt` | 新增 | 实现，通过 `ServiceManager.getService("amctl_service")` 获取 Binder |
| `app/.../di/AppModule.kt` | 修改 | 注册 AmctlServiceClient 的 DI 绑定 |

```kotlin
// AmctlServiceClient.kt
interface AmctlServiceClient {
    fun isAvailable(): Boolean
    fun createDisplay(width: Int, height: Int, dpi: Int): Int
    fun destroyDisplay(displayId: Int)
    fun listDisplays(): IntArray
    fun launchApp(displayId: Int, packageName: String)
    fun injectTap(displayId: Int, x: Float, y: Float)
    fun injectSwipe(displayId: Int, x1: Float, y1: Float, x2: Float, y2: Float, durationMs: Long)
    fun injectKeyEvent(displayId: Int, keyCode: Int)
    fun injectText(displayId: Int, text: String)
    fun captureFrame(displayId: Int, maxWidth: Int, maxHeight: Int, quality: Int): ByteArray?
    fun getUiTree(displayId: Int): String?
    fun findNodes(displayId: Int, by: String, value: String, exactMatch: Boolean): String?
    fun pressBack(displayId: Int)
    fun pressHome(displayId: Int)
}
```

**DoD**：App 进程能通过 AIDL 成功调用系统服务的所有方法。

### 任务 P5.2：MCP 工具路由层

| 文件 | 操作 | 说明 |
|------|------|------|
| `app/.../mcp/routing/ToolRouter.kt` | 新增 | 根据 `display_id` 参数路由到 v1 或 v2 路径 |

路由逻辑：
```
MCP Tool 调用
    │
    ├── arguments 中有 display_id? ──→ AmctlServiceClient (v2: AIDL → system_server)
    │
    └── arguments 中无 display_id ──→ AccessibilityService (v1: 物理屏幕)
```

所有现有工具无需修改——仅在调用链前端加路由判断。

**DoD**：编译通过，路由逻辑单元测试通过。

### 任务 P5.3：新增 v2 专属 MCP 工具

| 文件 | 操作 | 新增工具 |
|------|------|---------|
| `app/.../mcp/tools/DisplayTools.kt` | 新增 | `amctl_create_display`、`amctl_destroy_display`、`amctl_list_displays` |
| `app/.../mcp/tools/AppTools.kt` | 新增 | `amctl_launch_app`、`amctl_get_top_activity` |
| `app/.../mcp/McpServer.kt` | 修改 | 注册新工具 |

工具参数定义：

| 工具 | 参数 | 返回 |
|------|------|------|
| `amctl_create_display` | `width` (int), `height` (int), `dpi` (int, 可选，默认 420) | `{ "display_id": 5 }` |
| `amctl_destroy_display` | `display_id` (int) | `{ "success": true }` |
| `amctl_list_displays` | — | `{ "displays": [{ "id": 5, "width": 1080, ... }] }` |
| `amctl_launch_app` | `display_id` (int), `package_name` (string) | `{ "success": true }` |
| `amctl_get_top_activity` | `display_id` (int) | `{ "activity": "com.example/.MainActivity" }` |

**DoD**：`tools/list` 返回 22 个工具（v1 的 17 个 + v2 新增 5 个）。

### 任务 P5.4：升级 v1 工具 — 增加 display_id 参数

| 文件 | 操作 | 说明 |
|------|------|------|
| `app/.../mcp/tools/ScreenIntrospectionTools.kt` | 修改 | `amctl_get_screen_state` 增加可选 `display_id` 参数 |
| `app/.../mcp/tools/TouchActionTools.kt` | 修改 | 所有触摸工具增加可选 `display_id` 参数 |
| `app/.../mcp/tools/NodeActionTools.kt` | 修改 | 所有节点工具增加可选 `display_id` 参数 |
| `app/.../mcp/tools/TextInputTools.kt` | 修改 | 所有文本工具增加可选 `display_id` 参数 |
| `app/.../mcp/tools/SystemActionTools.kt` | 修改 | `press_back`、`press_home` 增加可选 `display_id` 参数 |
| `app/.../mcp/tools/UtilityTools.kt` | 修改 | 等待工具增加可选 `display_id` 参数 |

修改模式统一：
```kotlin
// 每个工具的参数 schema 中增加
"display_id" to JsonObject(mapOf(
    "type" to JsonPrimitive("integer"),
    "description" to JsonPrimitive("虚拟 Display ID（不传则操控物理屏幕）")
))

// 工具执行逻辑中
val displayId = arguments["display_id"]?.jsonPrimitive?.intOrNull
if (displayId != null) {
    // v2 路径：通过 AmctlServiceClient
    amctlServiceClient.injectTap(displayId, x, y)
} else {
    // v1 路径：通过 AccessibilityService（保持不变）
    actionExecutor.tap(x, y)
}
```

**DoD**：
```bash
# v1 路径依然工作（不传 display_id）
curl -X POST http://device:8080/mcp \
  -d '{"method":"tools/call","params":{"name":"amctl_tap","arguments":{"x":540,"y":1200}}}'
# 物理屏幕上执行

# v2 路径（传 display_id）
curl -X POST http://device:8080/mcp \
  -d '{"method":"tools/call","params":{"name":"amctl_tap","arguments":{"display_id":5,"x":540,"y":1200}}}'
# 虚拟 Display 上执行，物理屏幕无变化
```

### 任务 P5.5：端到端集成验证

完整工作流测试：

```bash
# 1. 创建虚拟 Display
curl ... '{"name":"amctl_create_display","arguments":{"width":1080,"height":2400}}'
# → display_id: 5

# 2. 启动微信
curl ... '{"name":"amctl_launch_app","arguments":{"display_id":5,"package_name":"com.tencent.mm"}}'

# 3. 获取屏幕状态（截图 + UI 树）
curl ... '{"name":"amctl_get_screen_state","arguments":{"display_id":5}}'

# 4. 点击
curl ... '{"name":"amctl_tap","arguments":{"display_id":5,"x":540,"y":1200}}'

# 5. 输入文本
curl ... '{"name":"amctl_type_text","arguments":{"display_id":5,"text":"你好"}}'

# 6. 全程检查物理屏幕 — 应无任何变化

# 7. 销毁
curl ... '{"name":"amctl_destroy_display","arguments":{"display_id":5}}'
```

**DoD**：完整工作流通过，物理屏幕全程无干扰。

---

## 阶段 P6：测试与优化（1-2 周）

**目标**：端到端测试、性能优化、SELinux 收紧、稳定性验证。

### 任务 P6.1：性能基准测试

| 指标 | 测试方法 | 目标 |
|------|---------|------|
| tap 延迟 | 注入 tap → App 响应（通过 UI 树变化检测） | <5ms |
| 截图延迟 | `captureFrame()` 耗时 | <10ms |
| UI 树读取延迟 | `getUiTree()` 耗时 | <50ms |
| 全流程延迟 | 截图 + UI 树 + tap | <200ms |
| 内存消耗 | 每个 VD 的增量内存 | <100MB |
| 并发 VD | 同时运行 4 个 VD 的稳定性 | 无 crash，无明显卡顿 |

```bash
# 简单性能测试脚本
for i in $(seq 1 100); do
    time adb shell service call amctl_service 20 i32 5 i32 720 i32 1280 i32 80
done
# 统计平均延迟
```

### 任务 P6.2：App 兼容性测试

| App | 测试项 | 预期问题 | 解决方案 |
|-----|--------|---------|---------|
| 微信 | 启动 + 聊天 + 朋友圈 | 可能检测非物理 Display | DisplayManagerService 伪装 |
| 支付宝 | 启动 + 首页浏览 | 同上 | 同上 |
| 设置 | 全功能 | 应完全正常 | — |
| Chrome | 网页浏览 | 应正常 | — |
| 相机 | 拍照 | VD 上无摄像头 | 不支持（预期行为） |

如果 App 检测 `Display.FLAG_PRIVATE` 或 `TYPE_VIRTUAL`，需要在 `DisplayManagerService` 中伪装为 `TYPE_BUILT_IN`。

### 任务 P6.3：SELinux 收紧

```bash
# 1. 收集 permissive 模式下的所有 amctl 相关 avc 日志
adb shell dmesg | grep "avc.*amctl"

# 2. 使用 audit2allow 生成最小策略
adb shell dmesg | audit2allow -p out/.../sepolicy

# 3. 将生成的策略写入 amctl_service.te
# 4. 编译 → 刷机 → 测试
# 5. 切换到 enforcing 模式
adb shell setenforce 1
```

**DoD**：在 enforcing 模式下所有功能正常。

### 任务 P6.4：稳定性测试

| 测试 | 方法 | 持续时间 |
|------|------|---------|
| 长时间运行 | 每 10 秒执行一次完整流程（截图 + tap） | 24 小时 |
| 反复创建/销毁 VD | 循环 create → launch → destroy | 1000 次 |
| 内存泄漏检查 | `dumpsys meminfo` 周期性采样 | 1 小时 |
| 异常恢复 | kill amctl 进程，检查 VD 是否清理 | — |

```bash
# 反复创建/销毁压力测试
for i in $(seq 1 1000); do
    display_id=$(adb shell service call amctl_service 1 i32 720 i32 1280 i32 320)
    adb shell service call amctl_service 5 i32 $display_id s16 "com.android.settings"
    sleep 2
    adb shell service call amctl_service 2 i32 $display_id
    echo "Round $i done"
done
```

**DoD**：24 小时稳定运行，无 crash、无内存泄漏、无 Display 残留。

### 任务 P6.5：文档更新

| 文件 | 操作 | 说明 |
|------|------|------|
| `docs/DESIGN-V2-ROM.md` | 更新 | 补充实际实现中的差异和经验 |
| `README.md` | 更新 | 增加 v2 功能介绍和使用说明 |
| `justfile` | 更新 | 增加 AOSP 相关的快捷命令 |

---

## 总体时间线

```
Week 1-2   ████████████████  P0: AOSP 环境搭建
Week 3-5   ████████████████████████  P1: 虚拟 Display
Week 6-7   ████████████████  P2: 输入注入
Week 8-9   ████████████████  P3: 帧捕获
Week 10    ████████          P4: UI 树
Week 11-12 ████████████████  P5: AIDL + MCP 整合
Week 13-14 ████████████████  P6: 测试与优化
```

**总计：8-14 周**（取决于 BSP 获取难度和编译环境就绪速度）

---

## 关键依赖与阻塞项

| 阻塞项 | 影响阶段 | 等级 | 应对 |
|--------|---------|------|------|
| BSP 源码获取 | P0 | **关键** | 第一优先级联系阿加犀 |
| 编译服务器 | P0 | **关键** | 本地笔记本不够，需租云或使用远程服务器 |
| 犀牛派设备到手 | P0.5 | 高 | 验证刷机需要实物设备 |
| 高通 vendor blob | P0.4 | 中 | 可从官方 ROM 提取 |
| InputDispatcher 是否需要修改 | P2 | 低 | 大概率不需要，AOSP 原生支持 displayId 路由 |

---

## 执行注意事项

1. **P0 是最大的不确定性** — BSP 获取和首次编译可能遇到各种问题，预留缓冲时间
2. **每个阶段独立验证** — 使用 `adb shell service call` 直接测试 AIDL，不要等到 P5 才发现底层问题
3. **增量编译优先** — 修改 Framework 后只编译 `framework-minus-apex services`（~5-15 分钟），不要全量编译
4. **SELinux 先 permissive** — P1-P5 阶段全部使用 permissive，P6 再收紧
5. **v1 保持不变** — v2 是扩展，不是替换。不传 `display_id` 时行为与 v1 完全一致
6. **补丁管理** — 所有 AOSP 修改以 `git format-patch` 形式保存在 `patches/` 目录，便于 rebase 到新版 AOSP
7. **日志约定** — 系统服务中使用 `Slog.d("AmctlService", ...)` 统一前缀，便于 logcat 过滤
8. **签名** — 编译使用 userdebug 签名，amctl App 使用平台签名（`platform.keystore`）以获得系统权限
