# amctl

[English](./README.md) | 中文

amctl 是一个 Android 设备控制服务，配套确定性 CLI 客户端 `amc`。

## 快速开始

### 1）在 Android 设备上部署服务

#### 方式 A：安装预编译 APK

安装 APK 并打开 App，然后完成以下步骤：

1. 为 amctl 开启无障碍权限。
2. 在首页打开 **服务** 开关。
3. 记录 App 中显示的连接信息：
   - 设备 IP
   - 端口
   - 令牌

#### 方式 B：本地源码构建并安装

```bash
just build
just install
```

然后按上面相同步骤完成配置。

### 2）安装并使用 `amc` CLI

源码构建：

```bash
cd cli
cargo build --release
```

设置环境变量（替换为你的真实值）：

```bash
export AMC_URL="http://<设备IP>:<端口>"
export AMC_TOKEN="<令牌>"
export AMC_DB="./amc.db"
```

执行首批命令：

```bash
./target/release/amc health
./target/release/amc observe top
./target/release/amc observe screen --max-rows 80 --fields id,text,desc,resId,flags
./target/release/amc act tap --x 540 --y 1200
```

## 常用 CLI 命令

```bash
amc observe screenshot --annotate --max-marks 120
amc verify text-contains --text "设置"
amc verify node-exists --by text --value "设置"
amc recover back --times 2
```

说明：

- 如果未设置 `AMC_URL`，则必须传 `--url`。
- 对受保护命令，如果未设置 `AMC_TOKEN`，则必须传 `--token`。
- 命令输出为单行 JSON。

## 常见问题

### 服务不可达

- 确认设备与主机在同一网络。
- 确认 App 中服务状态为运行中。
- 确认 `AMC_URL` 使用的是 App 显示的设备 IP。

### 认证失败

- 确认 `AMC_TOKEN` 与 App 显示令牌一致。
- 在 App 重新生成令牌并更新本地环境变量。

### 无障碍相关失败

- 在系统设置中重新检查无障碍权限。
- 回到 App 确认服务仍在运行。

## 开发者入口

```bash
just check
just build
cd cli && cargo test
```

更多文档：

- [CLI 说明](./cli/README.md)
- [设计文档](./docs/)
