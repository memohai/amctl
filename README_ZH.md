# Autofish

[English](./README.md) | 中文

Autofish 用来控制 Android 设备，配套的命令行工具是 `af`。

## 快速开始

### 1）从 GitHub Releases 安装 App（APK）

先到 GitHub Releases 下载最新 APK 并安装：  
https://github.com/memohai/Autofish/releases

打开 App 后按下面做：

1. 为 Autofish 开启无障碍权限。
2. 在首页打开 **服务** 开关。
3. 记下 App 里显示的连接信息：
   - 设备 IP
   - 端口
   - 令牌

### 2）通过 npm 安装 `af` CLI

```bash
npm i -g @memohjs/af
```

安装后先确认命令可用：

```bash
af --help
```

再设置环境变量：

```bash
export AF_URL="http://<设备IP>:<端口>"
export AF_TOKEN="<令牌>"
export AF_DB="./af.db"
```

可以先跑这几条命令：

```bash
af health
af observe top
af observe screen --max-rows 80 --fields id,text,desc,resId,flags
af observe refs --max-rows 80
af act tap --x 540 --y 1200
af act tap --by text --value "设置"
```

## 源码构建要求（可选）

如果你想自己从源码构建 APK 或 CLI，需要先准备：

- JDK 17
- Android SDK（包含 `adb`）
- Rust 工具链（`cargo`）
- `just`

建议设置：

- `ANDROID_HOME` 指向你的 Android SDK 路径

本地构建示例：

```bash
just build
just install
cd cli
cargo build --release
```

## 常用 CLI 命令

```bash
af observe screenshot --annotate --max-marks 120
af act swipe 100,1200,900,1200 --duration 300
af act tap --by resid --value "com.android.settings:id/title" --exact-match
af observe refs --max-rows 120
af act tap --by ref --value @n3
af verify text-contains --text "设置"
af verify node-exists --by text --value "设置"
af recover back --times 2
```

说明：

- 没有设置 `AF_URL` 或 `AF_TOKEN` 时，需要显式传 `--url` / `--token`。
- 命令输出为单行 JSON。

## 开发者入口

```bash
just check
just build
cd cli && cargo test
```

更多文档：

- [CLI 说明](./cli/README.md)
- [设计文档](./docs/)
