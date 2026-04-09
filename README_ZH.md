# Autofish

[English](./README.md) | 中文

Autofish 用来控制 Android 设备，配套的命令行工具是 `af`。

## 快速开始

### 1）从 GitHub Releases 安装 App（APK）

从 [Releases](https://github.com/memohai/Autofish/releases) 下载并安装最新 APK。

打开 App 后按下面做：

1. 为 Autofish 开启无障碍权限。
2. 启用 Shizuku 支持。
3. 在首页打开 **服务** 开关。
4. 记下 App 里显示的连接信息：
   - IP
   - PORT
   - TOKEN

### 2）通过 npm 安装 `af` CLI

```bash
npm i -g @memohjs/af
```

先用 `af config` 写入配置：

```bash
af config set remote.url "http://<IP>:<PORT>"
af config set remote.token "<TOKEN>"
af config set memory.db "$HOME/.config/af/af.db"
af config set output.default "text"
af config set artifacts.dir "$HOME/.config/af/artifacts"
```

先运行这些命令：

```bash
af health
af observe page --field screen --field refs --max-rows 80
af memory search --app com.android.settings
af memory context
af act tap --by ref --value @n1
af act tap --by text --value "设置"
af act tap --xy 540,1200
```

## 源码构建要求（可选）

如果你想自己从源码构建 APK 或 CLI，需要先准备：

- JDK 17
- Android SDK API 36
- Rust 工具链（`cargo`）
- `just`

本地构建示例：

```bash
just build
just install
```

更多文档：

- [CLI 说明](./cli/README.md)
- [Memory 说明](./docs/CLI_MEMORY.md)
- [设计文档](./docs/)
