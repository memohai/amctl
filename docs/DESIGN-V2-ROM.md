# amctl v2 — 定制 ROM 方案技术设计

## 1. 背景与动机

### 1.1 v1 方案（Accessibility Service）的局限

amctl v1 基于 Android Accessibility Service API 实现远程控制，存在以下固有限制：

| 限制 | 根因 | 影响 |
|------|------|------|
| 抢占屏幕 | 所有操作在物理屏幕的前台 Activity 上执行 | 用户无法同时使用手机 |
| 操作延迟高 | `dispatchGesture()` 经过 InputManager 排队 + 动画等待 | 单次交互 200-500ms |
| 截图慢 | `AccessibilityService.takeScreenshot()` 走 SurfaceFlinger 合成 | 200-500ms/帧 |
| 串行执行 | 一次只能操控一个前台 App | 无法并行操控多个 App |
| 节点信息滞后 | `AccessibilityNodeInfo` 依赖系统事件回调 | 几十到几百毫秒延迟 |
| 部分 App 防护 | 微信等 App 检测无障碍服务并限制功能 | 关键场景不可用 |

### 1.2 v2 目标

实现与豆包手机同等级的能力：

- **不抢屏幕**：AI 操控在虚拟 Display 上执行，用户在物理屏幕上正常使用
- **毫秒级响应**：输入注入 <5ms，帧捕获 <3ms
- **并行控制**：同时在多个虚拟 Display 上操控不同 App
- **静默执行**：用户完全无感知

### 1.3 为什么选择定制 ROM

| 方案 | 虚拟 Display 运行 Activity | 输入隔离 | GPU 帧直读 | App 兼容性 |
|------|---------------------------|---------|-----------|-----------|
| Accessibility Service (v1) | ✗ | ✗ | ✗ | 中 |
| Shizuku (shell 权限) | 受限 | 部分 | ✗ | 中 |
| Root + Magisk | ✗ | 部分 | 不稳定 | 低（检测） |
| **定制 ROM** | **✓** | **✓** | **✓** | **高** |

犀牛派 X1 基于高通 QCS8550（骁龙 8 Gen 2 同款，4nm ARM64），16GB LPDDR5X + 128GB UFS，
Adreno 740 GPU 和 48 TOPS NPU 提供充足算力。高通平台 AOSP 生态成熟，BSP 可从厂商或
CodeLinaro/CAF 获取，天然适合 ROM 定制。定制 ROM 的系统级控制能力（虚拟 Display、输入隔离、
GPU 帧直读）是 Magisk 方案无法稳定实现的，因此选择 C 路径。

---

## 2. 系统架构

### 2.1 总体架构

```
┌────────────────────────────────────────────────────────────────────┐
│                        amctl v2 系统                               │
│                                                                    │
│  ┌──────────────┐     ┌────────────────────────────────────────┐  │
│  │  物理 Display │     │         虚拟 Display Pool              │  │
│  │  (用户使用)   │     │  ┌──────┐ ┌──────┐ ┌──────┐          │  │
│  │              │     │  │ VD-1 │ │ VD-2 │ │ VD-N │          │  │
│  │  用户 App    │     │  │微信   │ │浏览器 │ │设置   │          │  │
│  │  正常运行    │     │  └──┬───┘ └──┬───┘ └──┬───┘          │  │
│  └──────┬───────┘     └─────┼───────┼───────┼────────────────┘  │
│         │                   │       │       │                    │
│  ┌──────┴───────────────────┴───────┴───────┴────────────────┐  │
│  │                    SurfaceFlinger                          │  │
│  │  PhysicalDisplay    VD-1 Layers   VD-2 Layers   ...       │  │
│  └────────────────────────────┬──────────────────────────────┘  │
│                               │                                  │
│  ┌────────────────────────────┴──────────────────────────────┐  │
│  │              AmctlSystemService (system_server)            │  │
│  │                                                            │  │
│  │  ┌─────────────────┐  ┌──────────────┐  ┌──────────────┐ │  │
│  │  │ DisplayController│  │InputInjector │  │ FrameCapture │ │  │
│  │  │                 │  │              │  │              │ │  │
│  │  │ create/destroy  │  │ tap/swipe/   │  │ GPU buffer   │ │  │
│  │  │ launch app      │  │ text/key     │  │ readback     │ │  │
│  │  │ manage lifecycle│  │ per-display  │  │ per-display  │ │  │
│  │  └─────────────────┘  └──────────────┘  └──────────────┘ │  │
│  │                                                            │  │
│  │  ┌──────────────┐  ┌──────────────┐  ┌────────────────┐  │  │
│  │  │ UiTreeReader │  │ AppManager   │  │ DisplayPool    │  │  │
│  │  │              │  │              │  │                │  │  │
│  │  │ UiAutomation │  │ install/     │  │ allocate/      │  │  │
│  │  │ per-display  │  │ uninstall/   │  │ reclaim/       │  │  │
│  │  │ dump         │  │ force-stop   │  │ resize         │  │  │
│  │  └──────────────┘  └──────────────┘  └────────────────┘  │  │
│  └────────────────────────────┬──────────────────────────────┘  │
│                               │ Binder IPC                       │
│  ┌────────────────────────────┴──────────────────────────────┐  │
│  │              MCP Server (App 进程)                         │  │
│  │                                                            │  │
│  │  Ktor HTTP (Netty) → BearerTokenAuth → MCP SDK Server     │  │
│  │                                                            │  │
│  │  工具分组:                                                  │  │
│  │  ├ display_*      — 虚拟 Display 生命周期                   │  │
│  │  ├ app_*          — App 启动/停止                           │  │
│  │  ├ input_*        — 输入注入 (tap/swipe/text/key)           │  │
│  │  ├ screen_*       — 帧捕获 + UI 树                         │  │
│  │  ├ system_*       — 系统操作 (back/home/recents)            │  │
│  │  └ util_*         — 等待/轮询                               │  │
│  └───────────────────────────────────────────────────────────┘  │
└────────────────────────────────────────────────────────────────────┘
```

### 2.2 组件职责

| 组件 | 层级 | 职责 |
|------|------|------|
| `AmctlSystemService` | system_server | 核心系统服务，通过 AIDL 暴露能力 |
| `DisplayController` | system_server | 虚拟 Display 的创建/销毁/配置 |
| `InputInjector` | system_server | 向指定 Display 注入触摸/按键事件 |
| `FrameCapture` | system_server | 从虚拟 Display 的 Surface 读取帧数据 |
| `UiTreeReader` | system_server | 通过 UiAutomation 读取指定 Display 的 UI 树 |
| `DisplayPool` | system_server | 管理虚拟 Display 池的分配/回收 |
| `MCP Server` | App 进程 | HTTP 服务器，将 MCP 协议翻译为 AIDL 调用 |

### 2.3 与 v1 的兼容

v2 保留 v1 的 Accessibility 通道作为 fallback：

```
MCP Tool 调用
    │
    ├── 指定了 displayId? ──→ AmctlSystemService (v2 路径)
    │
    └── 未指定 displayId ──→ AccessibilityService (v1 路径，物理屏幕)
```

---

## 3. 核心模块设计

### 3.1 虚拟 Display 管理

#### 3.1.1 创建流程

```
AmctlSystemService.createDisplay(width, height, dpi)
    │
    ├── 1. DisplayManagerService.createVirtualDisplay()
    │      flags: OWN_CONTENT_ONLY | PUBLIC | SUPPORTS_TOUCH
    │      绑定 Surface（用于帧捕获）
    │
    ├── 2. WindowManagerService 自动创建 DisplayContent
    │      包含完整的 TaskStack / ActivityStack
    │
    ├── 3. InputManagerService 为该 Display 分配 InputChannel
    │      独立的 InputDispatcher 焦点
    │
    └── 4. 返回 displayId
```

#### 3.1.2 AOSP 修改点

**`DisplayManagerService.java`**

```java
// frameworks/base/services/core/java/com/android/server/display/DisplayManagerService.java
//
// 修改: 允许系统服务创建带 SUPPORTS_TOUCH 标志的 VirtualDisplay
// 原因: 默认情况下只有 MediaProjection 持有者可以创建触摸可用的 VirtualDisplay

public int createAmctlDisplay(int width, int height, int densityDpi, Surface surface) {
    // 使用 VIRTUAL_DISPLAY_FLAG_PUBLIC 使其对所有 App 可见
    // 使用 VIRTUAL_DISPLAY_FLAG_OWN_CONTENT_ONLY 防止镜像物理屏
    // 使用 VIRTUAL_DISPLAY_FLAG_SUPPORTS_TOUCH 启用触摸输入
    int flags = DisplayManager.VIRTUAL_DISPLAY_FLAG_PUBLIC
              | DisplayManager.VIRTUAL_DISPLAY_FLAG_OWN_CONTENT_ONLY
              | DisplayManager.VIRTUAL_DISPLAY_FLAG_SUPPORTS_TOUCH;

    VirtualDisplay vd = mDisplayManager.createVirtualDisplay(
        "amctl-display-" + mNextDisplayIndex++,
        width, height, densityDpi,
        surface, flags);

    return vd.getDisplay().getDisplayId();
}
```

**`ActivityStarter.java`**

```java
// frameworks/base/services/core/java/com/android/server/wm/ActivityStarter.java
//
// 修改: 放行 amctl 系统服务向虚拟 Display 启动 Activity 的权限检查
// 原因: 默认只有 shell (uid=2000) 和 root 可以跨 Display 启动

private boolean isAllowedToStartOnDisplay(int callingUid, int targetDisplayId) {
    // 原有逻辑...

    // 新增: amctl 系统服务 (运行在 system_server 中) 始终允许
    if (isAmctlManagedDisplay(targetDisplayId)) {
        return true;
    }

    // 原有逻辑继续...
}
```

**`InputDispatcher.cpp`**

```cpp
// frameworks/native/services/inputflinger/dispatcher/InputDispatcher.cpp
//
// 修改: 确保注入的事件根据 displayId 正确路由
// 验证: 已有逻辑应该支持，需要确认 INJECT_INPUT_EVENT_MODE_ASYNC
//       在指定 displayId 时不会回退到默认 Display

void InputDispatcher::injectInputEvent(const InputEvent* event, ...) {
    // 关键: event->getDisplayId() 必须被保留并用于查找目标窗口
    // 不能 fallback 到 focused display
}
```

#### 3.1.3 Display 生命周期

```
            create
IDLE ──────────────────→ READY
                           │
                     launchApp
                           │
                           ▼
                        ACTIVE ←──── App 在此 Display 上运行
                           │
                      destroyDisplay
                           │
                           ▼
                       RELEASED ────→ Surface/Buffer 释放
```

#### 3.1.4 Display 池管理

```java
public class DisplayPool {
    private static final int MAX_DISPLAYS = 8;
    private final SparseArray<DisplaySlot> slots = new SparseArray<>();

    public synchronized int acquire(int width, int height, int dpi) {
        if (slots.size() >= MAX_DISPLAYS) {
            // LRU 淘汰最久未使用的 Display
            evictLeastRecentlyUsed();
        }
        int displayId = displayController.createDisplay(width, height, dpi);
        slots.put(displayId, new DisplaySlot(displayId, SystemClock.uptimeMillis()));
        return displayId;
    }

    public synchronized void release(int displayId) {
        displayController.destroyDisplay(displayId);
        slots.remove(displayId);
    }
}
```

---

### 3.2 输入注入

#### 3.2.1 注入方式对比

| 方式 | 延迟 | 隔离性 | 需要的权限 |
|------|------|--------|-----------|
| `AccessibilityService.dispatchGesture()` | 200-500ms | 无（影响物理屏幕） | 无障碍服务 |
| `adb shell input` | 50-100ms | 无（fork 开销） | shell |
| `InputManager.injectInputEvent()` | 1-5ms | 按 displayId 路由 | `INJECT_EVENTS`（系统签名） |
| `/dev/input/eventX` 直写 | <1ms | 无（混入全局流） | root |

v2 使用 `InputManager.injectInputEvent()`，在系统服务中调用，无需额外权限。

#### 3.2.2 实现

```java
public class AmctlInputInjector {
    private final InputManager inputManager;

    public void injectTap(int displayId, float x, float y) {
        long downTime = SystemClock.uptimeMillis();

        MotionEvent down = createMotionEvent(downTime, downTime,
            MotionEvent.ACTION_DOWN, x, y, displayId);
        inputManager.injectInputEvent(down,
            InputManager.INJECT_INPUT_EVENT_MODE_ASYNC);
        down.recycle();

        long upTime = downTime + 50;
        MotionEvent up = createMotionEvent(downTime, upTime,
            MotionEvent.ACTION_UP, x, y, displayId);
        inputManager.injectInputEvent(up,
            InputManager.INJECT_INPUT_EVENT_MODE_ASYNC);
        up.recycle();
    }

    public void injectSwipe(int displayId,
            float x1, float y1, float x2, float y2, long durationMs) {
        long downTime = SystemClock.uptimeMillis();
        int steps = Math.max((int)(durationMs / 16), 1);  // ~60fps

        // DOWN
        inject(createMotionEvent(downTime, downTime,
            MotionEvent.ACTION_DOWN, x1, y1, displayId));

        // MOVE 序列
        for (int i = 1; i <= steps; i++) {
            float fraction = (float) i / steps;
            float x = x1 + (x2 - x1) * fraction;
            float y = y1 + (y2 - y1) * fraction;
            long eventTime = downTime + (durationMs * i / steps);
            inject(createMotionEvent(downTime, eventTime,
                MotionEvent.ACTION_MOVE, x, y, displayId));
            SystemClock.sleep(16);
        }

        // UP
        inject(createMotionEvent(downTime, downTime + durationMs,
            MotionEvent.ACTION_UP, x2, y2, displayId));
    }

    public void injectText(int displayId, String text) {
        // 方案一：逐字符发送 KeyEvent（兼容性好）
        // 方案二：通过 InputMethodManager 注入到目标窗口的 InputConnection
        // 方案三：使用 ClipboardManager + paste（最快）

        // 推荐方案二，需要在 InputMethodManagerService 中添加接口
        InputConnection ic = getTargetInputConnection(displayId);
        if (ic != null) {
            ic.commitText(text, 1);
        }
    }

    public void injectKeyEvent(int displayId, int keyCode) {
        long now = SystemClock.uptimeMillis();
        KeyEvent down = new KeyEvent(now, now, KeyEvent.ACTION_DOWN, keyCode, 0);
        down.setDisplayId(displayId);
        inputManager.injectInputEvent(down, InputManager.INJECT_INPUT_EVENT_MODE_ASYNC);

        KeyEvent up = new KeyEvent(now, now + 50, KeyEvent.ACTION_UP, keyCode, 0);
        up.setDisplayId(displayId);
        inputManager.injectInputEvent(up, InputManager.INJECT_INPUT_EVENT_MODE_ASYNC);
    }

    private MotionEvent createMotionEvent(
            long downTime, long eventTime, int action,
            float x, float y, int displayId) {
        MotionEvent.PointerProperties[] props = new MotionEvent.PointerProperties[1];
        props[0] = new MotionEvent.PointerProperties();
        props[0].id = 0;
        props[0].toolType = MotionEvent.TOOL_TYPE_FINGER;

        MotionEvent.PointerCoords[] coords = new MotionEvent.PointerCoords[1];
        coords[0] = new MotionEvent.PointerCoords();
        coords[0].x = x;
        coords[0].y = y;
        coords[0].pressure = 1.0f;
        coords[0].size = 1.0f;

        MotionEvent event = MotionEvent.obtain(
            downTime, eventTime, action, 1, props, coords,
            0, 0, 1.0f, 1.0f, 0, 0,
            InputDevice.SOURCE_TOUCHSCREEN, 0);
        event.setDisplayId(displayId);
        return event;
    }
}
```

#### 3.2.3 输入隔离验证

```
物理屏幕 (displayId=0):  用户触摸 → InputDispatcher → 物理窗口
虚拟屏幕 (displayId=5):  injectInputEvent(displayId=5) → InputDispatcher → 虚拟窗口

两条输入管道完全隔离，互不干扰。
```

---

### 3.3 帧捕获

#### 3.3.1 方案对比

| 方案 | 延迟 | 内存拷贝 | 适用场景 |
|------|------|---------|---------|
| `AccessibilityService.takeScreenshot()` | 200-500ms | 2次 | v1 fallback |
| `MediaProjection + ImageReader` | 30-50ms | 1次 | 需要用户授权弹窗 |
| `VirtualDisplay + ImageReader` | 5-10ms | 零拷贝(GPU) | v2 主方案 |
| `SurfaceControl.screenshot()` | 3-5ms | 1次 | 按需截图 |
| `PixelCopy` | 1-3ms | GPU→CPU | 单窗口截图 |

#### 3.3.2 主方案：VirtualDisplay + ImageReader

```java
public class AmctlFrameCapture {
    private final SparseArray<CaptureSession> sessions = new SparseArray<>();

    public void attachToDisplay(int displayId, int width, int height) {
        ImageReader reader = ImageReader.newInstance(
            width, height, PixelFormat.RGBA_8888, 2,
            HardwareBuffer.USAGE_GPU_SAMPLED_IMAGE
                | HardwareBuffer.USAGE_CPU_READ_OFTEN);

        // 将 ImageReader 的 Surface 绑定到虚拟 Display
        // 每一帧 App 渲染都直接写入这个 Surface 的 BufferQueue
        displayController.setSurface(displayId, reader.getSurface());

        // 可选：注册帧回调（实时流）
        reader.setOnImageAvailableListener(r -> {
            Image image = r.acquireLatestImage();
            if (image != null) {
                onFrameAvailable(displayId, image);
                image.close();
            }
        }, backgroundHandler);

        sessions.put(displayId, new CaptureSession(reader));
    }

    // 按需截图（MCP 工具调用时）
    public ScreenshotResult captureFrame(int displayId, int maxWidth, int maxHeight, int quality) {
        CaptureSession session = sessions.get(displayId);
        if (session == null) return null;

        Image image = session.reader.acquireLatestImage();
        if (image == null) return null;

        try {
            // Image → Bitmap（GPU buffer 直读，~1ms）
            HardwareBuffer hwBuffer = image.getHardwareBuffer();
            Bitmap bitmap = Bitmap.wrapHardwareBuffer(hwBuffer, null);

            // 缩放（如果需要）
            if (bitmap.getWidth() > maxWidth || bitmap.getHeight() > maxHeight) {
                bitmap = scaleBitmap(bitmap, maxWidth, maxHeight);
            }

            // 编码 JPEG
            ByteArrayOutputStream baos = new ByteArrayOutputStream();
            bitmap.compress(Bitmap.CompressFormat.JPEG, quality, baos);

            return new ScreenshotResult(
                Base64.encodeToString(baos.toByteArray(), Base64.NO_WRAP),
                bitmap.getWidth(), bitmap.getHeight());
        } finally {
            image.close();
        }
    }
}
```

#### 3.3.3 帧捕获流水线

```
App 渲染 (GPU)
    │
    ▼
BufferQueue (Surface)
    │
    ▼
ImageReader.acquireLatestImage()     ← 零拷贝，直接拿 GPU buffer 引用
    │
    ├─→ 实时流模式: onFrameAvailable 回调
    │
    └─→ 按需模式: captureFrame() 调用时 acquire
            │
            ▼
        Bitmap.wrapHardwareBuffer()   ← 零拷贝，Bitmap 包装 HardwareBuffer
            │
            ▼
        JPEG 编码 + Base64            ← 唯一的 CPU 开销
            │
            ▼
        MCP 响应返回
```

---

### 3.4 UI 树读取

#### 3.4.1 方案

系统服务中使用 `UiAutomation` API 按 Display 读取 UI 树，替代 Accessibility Service 的节点遍历：

```java
public class AmctlUiTreeReader {

    // 通过 AccessibilityManagerService 获取指定 Display 的窗口列表
    public String dumpUiTree(int displayId) {
        List<AccessibilityWindowInfo> windows =
            accessibilityManager.getWindowsOnDisplay(displayId);

        StringBuilder tsv = new StringBuilder();
        for (AccessibilityWindowInfo window : windows) {
            AccessibilityNodeInfo root = window.getRoot();
            if (root != null) {
                dumpNode(tsv, root, 0);
                root.recycle();
            }
        }
        return tsv.toString();
    }

    private void dumpNode(StringBuilder sb, AccessibilityNodeInfo node, int depth) {
        // 与 v1 的 CompactTreeFormatter 格式一致
        // node_id, class, text, desc, res_id, bounds, flags
    }
}
```

#### 3.4.2 与 v1 格式兼容

v2 的 UI 树输出保持与 v1 完全一致的 TSV 格式，LLM 端无需修改 prompt。

---

### 3.5 AIDL 接口定义

```aidl
// frameworks/base/core/java/android/amctl/IAmctlService.aidl

package android.amctl;

interface IAmctlService {
    // === Display 管理 ===
    int createDisplay(int width, int height, int densityDpi);
    void destroyDisplay(int displayId);
    void resizeDisplay(int displayId, int width, int height, int densityDpi);
    int[] listDisplays();

    // === App 管理 ===
    void launchApp(int displayId, String packageName);
    void launchIntent(int displayId, in Intent intent);
    void forceStopApp(int displayId, String packageName);
    String getTopActivity(int displayId);

    // === 输入注入 ===
    void injectTap(int displayId, float x, float y);
    void injectLongPress(int displayId, float x, float y, long durationMs);
    void injectDoubleTap(int displayId, float x, float y);
    void injectSwipe(int displayId, float x1, float y1, float x2, float y2, long durationMs);
    void injectKeyEvent(int displayId, int keyCode);
    void injectText(int displayId, String text);

    // === 帧捕获 ===
    byte[] captureFrame(int displayId, int maxWidth, int maxHeight, int quality);
    void startFrameStream(int displayId, int fps, IFrameCallback callback);
    void stopFrameStream(int displayId);

    // === UI 树 ===
    String getUiTree(int displayId);
    String findNodes(int displayId, String by, String value, boolean exactMatch);

    // === 全局操作 ===
    void pressBack(int displayId);
    void pressHome(int displayId);
    void pressRecents(int displayId);
}

interface IFrameCallback {
    void onFrame(in byte[] jpegData, long timestampMs);
}
```

---

## 4. MCP 工具设计（v2）

### 4.1 新增工具

| 工具 | 描述 | 参数 |
|------|------|------|
| `amctl_create_display` | 创建虚拟 Display | `width`, `height`, `dpi` (可选) |
| `amctl_destroy_display` | 销毁虚拟 Display | `display_id` |
| `amctl_list_displays` | 列出所有虚拟 Display | — |
| `amctl_launch_app` | 在指定 Display 上启动 App | `display_id`, `package_name` |
| `amctl_get_top_activity` | 获取 Display 上的顶层 Activity | `display_id` |

### 4.2 升级工具（增加 display_id 参数）

所有 v1 工具增加可选的 `display_id` 参数。不传时走 v1 路径（物理屏幕 + Accessibility）。

| 工具 | 新增参数 | 行为变化 |
|------|---------|---------|
| `amctl_get_screen_state` | `display_id` (可选) | 传了走 FrameCapture + UiTreeReader；不传走 Accessibility |
| `amctl_tap` | `display_id` (可选) | 传了走 InputInjector；不传走 dispatchGesture |
| `amctl_swipe` | `display_id` (可选) | 同上 |
| `amctl_type_text` | `display_id` (可选) | 传了走 InputConnection 注入 |
| `amctl_press_back` | `display_id` (可选) | 传了走 injectKeyEvent |
| ... | ... | ... |

### 4.3 典型工作流

```
LLM:
  1. amctl_create_display(1080, 2400, 420)  → display_id: 5
  2. amctl_launch_app(5, "com.tencent.mm")  → 微信在 VD-5 上启动
  3. amctl_get_screen_state(display_id=5)   → 微信的 UI 树 + 截图
  4. amctl_tap(display_id=5, x=540, y=1200) → 在 VD-5 上点击
  5. amctl_type_text(display_id=5, ...)      → 在 VD-5 上输入

用户全程在物理屏幕上正常使用手机，完全无感知。
```

---

## 5. AOSP 修改清单

### 5.1 Framework 层 (Java)

| 文件 | 修改类型 | 说明 |
|------|---------|------|
| `services/core/.../AmctlSystemService.java` | 新增 | 核心系统服务 |
| `services/core/.../amctl/DisplayController.java` | 新增 | 虚拟 Display 管理 |
| `services/core/.../amctl/InputInjector.java` | 新增 | 输入注入 |
| `services/core/.../amctl/FrameCapture.java` | 新增 | 帧捕获 |
| `services/core/.../amctl/UiTreeReader.java` | 新增 | UI 树读取 |
| `services/core/.../amctl/DisplayPool.java` | 新增 | Display 池管理 |
| `services/java/.../SystemServer.java` | 修改 | 注册 AmctlSystemService |
| `services/core/.../wm/ActivityStarter.java` | 修改 | 放行跨 Display 启动 |
| `services/core/.../display/DisplayManagerService.java` | 修改 | 支持创建可交互 VirtualDisplay |
| `core/java/android/amctl/IAmctlService.aidl` | 新增 | AIDL 接口定义 |

### 5.2 Native 层 (C++)

| 文件 | 修改类型 | 说明 |
|------|---------|------|
| `native/services/inputflinger/.../InputDispatcher.cpp` | 验证 | 确认 displayId 路由正确 |

### 5.3 SELinux 策略

```
# device/qualcomm/rhinopi_x1/sepolicy/amctl_service.te
# 高通 QCS8550 平台 SELinux 策略

type amctl_service, domain;
type amctl_service_exec, exec_type, file_type;

# 允许 amctl_service 创建虚拟 Display
allow amctl_service surfaceflinger_service:service_manager find;
allow amctl_service display_service:service_manager find;

# 允许 amctl_service 注入输入事件
allow amctl_service input_service:service_manager find;
allow amctl_service inputflinger_service:binder call;

# 允许 amctl_service 访问 Adreno 740 GPU buffer
allow amctl_service gpu_device:chr_file rw_file_perms;
allow amctl_service ion_device:chr_file rw_file_perms;
allow amctl_service dmabuf_system_heap_device:chr_file rw_file_perms;
```

---

## 6. 构建与部署

### 6.1 目标硬件规格

| 项目 | 规格 |
|------|------|
| 开发板 | 犀牛派 X1 (RhinoPi X1) |
| SoC | 高通 QCS8550（骁龙 8 Gen 2 同款，4nm） |
| 架构 | **ARM64** (aarch64) |
| CPU | 6 核 Kryo：1x 3.2GHz + 2x 2.8GHz + 3x 2.0GHz |
| GPU | Adreno 740（4.4 TFLOPS FP16） |
| NPU | ~48 TOPS INT8 |
| 内存 | 16GB LPDDR5X |
| 存储 | 128GB UFS |
| 无线 | Wi-Fi 7、BT 5.3/BLE |
| 接口 | USB 3.0/2.0/USB-C/HDMI/RJ45/40-PIN GPIO |
| 刷机模式 | 高通 EDL（9008 端口） + QFIL |
| 官方系统 | AidLux 融合系统（Android + Ubuntu 22.04） |

### 6.2 开发环境要求

| 资源 | 要求 |
|------|------|
| CPU | 16 核以上（AOSP 全量编译） |
| 内存 | 64GB 以上 |
| 磁盘 | 500GB SSD（AOSP 源码 ~200GB + 编译产物 ~150GB） |
| OS | Ubuntu 22.04 / 24.04 |
| JDK | OpenJDK 17 |
| Python | 3.10+ |
| 刷机工具 | QPST 2.7.496 + 高通 USB 驱动（Windows），或 qdl (Linux) |

### 6.3 构建流程

```bash
# 1. 获取 AOSP 源码（高通 QCS8550 基于 Android 14）
#    优先从 CodeLinaro/CAF 获取高通专用分支
repo init -u https://git.codelinaro.org/clo/la/platform/manifest \
    -b release -m LA.QSSI.14.0.r1-xxxxx-qssi.0.xml
repo sync -j16

# 2. 合并犀牛派 BSP（需从阿加犀获取）
#    包含 device/qualcomm/rhinopi_x1/、kernel/msm-5.15/、vendor/qcom/ 等
#    如果阿加犀提供完整 AOSP 源码包，可直接使用

# 3. 应用 amctl 补丁
git am patches/0001-amctl-system-service.patch
git am patches/0002-amctl-display-controller.patch
git am patches/0003-amctl-input-injector.patch
git am patches/0004-amctl-frame-capture.patch

# 4. 编译
source build/envsetup.sh
lunch rhinopi_x1-userdebug
make -j$(nproc)
```

### 6.4 刷机流程

犀牛派 X1 使用高通 EDL (Emergency Download) 模式刷机，**不支持 fastboot flashall**。

#### 方式一：QFIL（Windows）

```bash
# 1. 进入 EDL 模式
adb reboot edl
# 设备出现 Qualcomm HS-USB QDLoader 9008 端口

# 2. 打开 QFIL（QPST 安装目录中的 QFIL.exe）
#    Configuration → FireHose Configuration:
#      Download Protocol: 0-Sahara
#      Device Type: ufs
#      勾选 Reset After Download

# 3. Select Port → 选择 9008 端口
# 4. Select Build Type → Flat Build
# 5. Programmer Path → 选择编译产物中的 xbl_s_devprg_ns.melf
# 6. Load XML → 选择刷机 xml 文件（rawprogram*.xml + patch*.xml）
# 7. Download → 等待 ~5 分钟完成
```

#### 方式二：qdl（Linux，推荐开发使用）

```bash
# 安装 qdl（Qualcomm Download tool for Linux）
git clone https://github.com/linux-msm/qdl.git
cd qdl && make && sudo make install

# 刷机（设备已进入 EDL 模式）
cd out/target/product/rhinopi_x1/
qdl prog_firehose_ddr.elf rawprogram*.xml patch*.xml
```

#### 方式三：adb sideload（增量更新，日常开发）

```bash
# 生成 OTA 包
make otapackage -j$(nproc)

# 通过 recovery 模式 sideload
adb reboot recovery
adb sideload out/target/product/rhinopi_x1/rhinopi_x1-ota-*.zip
```

### 6.5 开发迭代

修改 Framework 后无需全量编译：

```bash
# 只编译 framework
make framework-minus-apex -j$(nproc)

# 推送到设备（需 userdebug/eng 版本）
adb root
adb remount
adb push out/target/product/rhinopi_x1/system/framework/framework.jar /system/framework/
adb push out/target/product/rhinopi_x1/system/framework/services.jar /system/framework/
adb reboot
```

---

## 7. 性能预期

### 7.1 延迟对比

| 操作 | v1 (Accessibility) | v2 (ROM 定制) | 提升倍数 |
|------|-------------------|---------------|---------|
| 单次 tap | 200-500ms | 1-5ms | 40-100x |
| 滑动手势 | 300-800ms | 50-100ms | 6-8x |
| 文本输入 | 100-300ms | 1-5ms | 20-60x |
| 截图 | 200-500ms | 1-3ms | 67-167x |
| UI 树读取 | 50-200ms | 10-50ms | 2-5x |
| 全流程 (截图+分析+操作) | 1-3s | 50-200ms | 5-15x |

### 7.2 资源消耗

| 资源 | 每个虚拟 Display |
|------|-----------------|
| 内存 | ~50-100MB（取决于 App 复杂度） |
| GPU | 一个额外的 render target |
| CPU | 仅在帧捕获/UI 树解析时短暂占用 |

犀牛派 X1 配备 16GB LPDDR5X + Adreno 740，建议最多同时运行 4-8 个虚拟 Display。

---

## 8. 实施计划

| 阶段 | 周期 | 里程碑 |
|------|------|--------|
| **P0: 环境搭建** | 1-2 周 | AOSP + QCS8550 BSP 编译通过，通过 QFIL/qdl 刷入犀牛派 X1 正常启动 |
| **P1: 虚拟 Display** | 2-3 周 | 可创建虚拟 Display 并在上面启动 Activity |
| **P2: 输入注入** | 1-2 周 | 可向虚拟 Display 注入 tap/swipe/key/text |
| **P3: 帧捕获** | 1-2 周 | 可从虚拟 Display 捕获帧数据（JPEG） |
| **P4: UI 树** | 1 周 | 可读取虚拟 Display 上 App 的 UI 树 |
| **P5: AIDL + MCP 整合** | 1-2 周 | 系统服务通过 AIDL 暴露，MCP Server 调用 |
| **P6: 测试与优化** | 1-2 周 | 端到端测试、性能优化、稳定性验证 |
| **总计** | **8-14 周** | |

### 8.1 每阶段验证标准

**P0**:
```bash
adb shell getprop ro.build.display.id  # 显示自定义 build ID
```

**P1**:
```bash
adb shell dumpsys display | grep "amctl-display"  # 虚拟 Display 存在
adb shell dumpsys activity activities | grep "displayId=5"  # Activity 在虚拟 Display 上
```

**P2**:
```bash
adb shell service call amctl_service 10 i32 5 f 540.0 f 1200.0  # AIDL 调用 injectTap
# 虚拟 Display 上的 App 响应点击
```

**P3**:
```bash
adb shell service call amctl_service 20 i32 5 i32 720 i32 1280 i32 80  # captureFrame
# 返回 JPEG 数据
```

**P5**:
```bash
curl -X POST http://device:8080/mcp \
  -d '{"method":"tools/call","params":{"name":"amctl_tap","arguments":{"display_id":5,"x":540,"y":1200}}}'
# MCP 调用成功，物理屏幕无任何变化
```

---

## 9. 风险与缓解

| 风险 | 可能性 | 影响 | 缓解措施 |
|------|--------|------|---------|
| 犀牛派 BSP 不提供完整 AOSP 源码 | 中 | 阻塞 P0 | 提前联系阿加犀（AidLux）获取 BSP；备选从 CodeLinaro/CAF 抽取高通 QCS8550 公版 BSP |
| 高通 vendor blob 不完整 | 中 | 编译/运行异常 | 从官方 ROM 中提取 vendor 分区镜像；使用 `extract-files.sh` 工具 |
| Adreno 740 GPU 闭源驱动与 HardwareBuffer 兼容性 | 低 | 帧捕获异常 | Adreno 系列在 AOSP 上兼容性成熟，fallback 到 `SurfaceControl.screenshot()` |
| EDL 刷机失败 / 变砖 | 低 | 设备不可用 | 掌握 EDL 9008 短接救砖方法；始终保留官方 ROM 备份 |
| 部分 App 检测非物理 Display 并拒绝运行 | 中 | 功能受限 | 在 DisplayManagerService 中伪装 Display 属性（TYPE_BUILT_IN） |
| VirtualDisplay 上的 App 收不到某些系统广播 | 低 | 功能异常 | 在 ActivityManagerService 中补发广播 |
| SELinux 策略过于复杂 | 中 | 调试困难 | 初期使用 permissive 模式，稳定后收紧 |
| 犀牛派融合系统（AidLux）与纯 AOSP 冲突 | 中 | 丢失 AidLux 容器功能 | 可在 AOSP 中保留 AidLux 启动脚本；或改用纯 AOSP 模式放弃 AidLux |

---

## 10. 未来扩展

| 能力 | 描述 |
|------|------|
| **多用户隔离** | 每个虚拟 Display 运行在不同 Android User Profile 下，数据隔离 |
| **视频流** | 通过 WebRTC/RTSP 实时推送虚拟 Display 的画面 |
| **VLM 集成** | 帧数据直接传给视觉语言模型，无需 UI 树 |
| **录屏回放** | 记录虚拟 Display 上的操作序列，支持回放和调试 |
| **云端调度** | 多台设备的虚拟 Display 统一管理，支持负载均衡 |
