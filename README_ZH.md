# Autofish

[English](./README.md) | 中文

**给 AI agent 用的确定性 Android 控制层。**

Autofish 给你的 agent 提供一个稳定、明确、可观测的 Android 设备接口。它不做任务规划，不编排工作流，不替你做决策——那是你的 agent 该做的事。Autofish 只管最后一公里：看屏幕、执行操作、验证结果、处理失败。

```text
人 → agent → autofish → Android 设备
      ↑                      |
      └── 观测 / 验证 ────────┘
```

## Why Autofish

多 agent 编排是一个快速演进的领域。Agent 之间如何协调、规划、重试、委托，终将收敛出最佳实践和专用平台——就像微服务编排收敛到了 Kubernetes。设备控制是另一个关注点，不应该与之纠缠。

当规划和执行被熔在一起，agent 失去对实际行为的控制权，失败被内部消化而非暴露，采用更好的编排策略也意味着重写整个栈。

Autofish 保持边界清晰：**设备控制是基础设施，不是应用逻辑。** 你的 agent——或者未来涌现出的任何编排层——保留对规划和决策的全部权力。Autofish 提供确定性的设备操作、明确的失败信号、和可查询的执行记录。当编排的故事成熟时，Autofish 不需要变。

## 设计原则

| 原则 | 在实践中的含义 |
|---|---|
| **单一职责** | 只做 Android 观测和控制。不做规划，不调 LLM，不搞工作流引擎。 |
| **Agent-first 接口** | CLI 和服务 API 为机器消费而设计：结构化输出、确定性退出码、可组合的命令。 |
| **确定性表面** | 命令做的就是它说的。没有隐式重试，没有背后的"智能"启发式。 |
| **闭环纪律** | 每次操作前必须观测，操作后必须验证。内存模型以此为纪律。 |
| **优雅降级** | 多条执行路径共存。当首选能力不可用时，回退路径保持系统可运行。 |
| **可观测的执行** | 每次操作、状态转换、恢复都被结构化记录——agent 运行中随时可查。 |

## 工作方式

Autofish 包含两个组件：

**Android 服务** — 设备上运行的前台服务，暴露 HTTP API。负责截图、UI 树检查、点击/滑动/文字输入执行、以及屏幕覆盖层标注。

**CLI (`af`)** — Rust 编写的命令行工具，通过 HTTP 与服务通信。提供命令接口，管理本地工具记忆（SQLite），处理产物存储。通过 npm（`@memohjs/af`）分发。

### 观测-行动-验证循环

```bash
af observe page --field screen --field refs --max-rows 80
af act tap --by ref --value @n3
af verify text-contains --text "Wi-Fi"
af memory save --app com.android.settings --topic "nav/home-to-wifi" \
  --content "从设置主页点击 @n3 (Wi-Fi)"
```

### 工具记忆

CLI 维护一个本地 SQLite 数据库，追踪：

- **事件** — 每次 `act`、`verify`、`recover`，附带页面指纹、状态、失败原因和耗时。
- **状态转换** — 自动闭合的 act → verify 对，带成功/失败计数。
- **恢复策略** — 哪些恢复步骤对哪些失败原因有效。
- **Agent 笔记** — 你的 agent 写入和查询的追加式知识。

## 文档

- [快速开始](./docs/QUICKSTART.md) — 从零到第一条命令
- [CLI 参考](./cli/README.md) — 全部命令、参数和输出格式
- [架构说明](./docs/ARCHITECTURE.md) — 服务内部、Refs 设计、API 约定
- [CLI 记忆](./docs/CLI_MEMORY.md) — 数据模型、记录规则、观测缓存
- [更新日志](./CHANGELOG.md) — 发布历史和破坏性变更

## 从源码构建

要求：JDK 17、Android SDK API 36、Rust 工具链、`just`。

```bash
just build
just install
```