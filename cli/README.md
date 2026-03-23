# amc CLI (Rust)

`amc` 是 `amctl` 的 deterministic CLI 执行器（Rust 版本）。

## 构建

```bash
cd cli
cargo build
```

## 运行

```bash
cargo run -- preflight --base-url http://127.0.0.1:8081
cargo run -- act tap --base-url http://127.0.0.1:8081 --token "$AMCTL_TOKEN" --x 540 --y 1200
```

## 命令集

- `preflight`
- `act tap` / `act swipe` / `act back` / `act home` / `act text` / `act launch` / `act stop` / `act key`
- `observe screen` / `observe screenshot` / `observe top`
- `verify text-contains` / `verify top-activity` / `verify node-exists`
- `recover back` / `recover home` / `recover relaunch`

## 输出格式

每条命令输出一行 JSON：

- 成功：`{"ok":true,"code":"OK",...}`
- 失败：`{"ok":false,"code":"...",...}`

支持 `Ctrl+C`（SIGINT）中断，返回 `code="INTERRUPTED"`，进程退出码为 `130`。
