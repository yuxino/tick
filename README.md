<div align="center">
  <img src="public/tick-icon.png" width="96" alt="Tick">
  <h1>Tick</h1>
  <p>把系统定时任务变成看得懂、改得动的自动化。</p>
  <p>
    <a href="https://github.com/yuxino/tick/releases/latest">旧版存档</a>
    · <a href="#开发">从源码运行</a>
    · <a href="https://github.com/yuxino/tick/issues">Issue</a>
  </p>
</div>

![Tick 主界面](docs/images/tick-overview.png)

## 能做什么

- 创建、编辑、停用、立即运行和删除任务
- 按每天、每月、每年或固定间隔调度
- 直接编写 Node.js，或填写 `.js` 文件路径并配置 Node 可执行文件
- 保存前试跑 Node.js，查看 stdout、stderr 与任务定义
- 在月历中检查固定时间任务
- 可选用 DeepSeek 从一句话生成可编辑草稿

macOS 使用当前用户 LaunchAgent，Windows 使用当前用户 Task Scheduler。Tick 只管理自己创建且带所有权标记的任务，不修改系统 daemon 或系统任务，也不请求管理员权限；任务以当前用户权限运行。Windows 的固定间隔为 60 秒至 31 天，超出范围会直接拒绝，不会静默取整；macOS 最短为 1 秒。调度环境不读取交互式 shell 的 profile，Node、脚本和工作目录建议填写绝对路径。AI 生成的内容只是一份草稿，保存前可检查和修改。

## 本地数据

| 内容 | 位置 |
| --- | --- |
| macOS 任务定义 | `~/Library/LaunchAgents/com.gavin.tick.*.plist` |
| macOS 脚本和日志 | `~/Library/Application Support/tick/` |
| Windows 任务 | 当前用户 Task Scheduler 中的 `Tick.job-*` |
| Windows 脚本、任务索引和日志 | `%APPDATA%\tick\` |
| macOS DeepSeek API Key | `~/Library/Application Support/com.gavin.tick/settings.json` |
| Windows DeepSeek API Key | `%APPDATA%\com.gavin.tick\settings.json` |

DeepSeek 完全可选。使用时，任务描述和 API Key 会发送到 `api.deepseek.com`；Key 以当前用户配置目录中的明文文件保存，可随时在设置中删除。macOS 目录和文件权限分别设为 `0700`、`0600`；Windows 依赖当前用户配置目录的访问控制，不属于加密存储。不用 DeepSeek 时，Tick 不需要任何 API Key。

## 开发

需要 Node.js 22 和 Rust。macOS 还需要 Xcode Command Line Tools；Windows 需要 Microsoft C++ Build Tools 和 WebView2。当前 CI 只验证 Apple 芯片（M 系列、arm64）Mac；Windows 11 ARM64 的原生验收仍在进行，Intel Mac 尚未验证。

```bash
npm install
npm run tauri dev
```

检查代码：

```bash
npm run check
```

构建应用：

```bash
npm run tauri build
```

当前源码包含 macOS LaunchAgent 与 Windows 当前用户 Task Scheduler 后端，但 Windows 11 ARM64 的原生验收尚未完成，也还没有发布 Windows 安装包。公开的 [v0.1.0 安装包](https://github.com/yuxino/tick/releases/tag/v0.1.0) 仅支持 Apple 芯片（M 系列）Mac 和 macOS 11 或更高版本，不支持 Intel Mac；它早于当前 `main`，应用包签名不完整，也没有 Developer ID 签名或 Apple 公证，因此只作为旧版存档。要使用当前版本，请从源码运行。贡献说明见 [CONTRIBUTING.md](CONTRIBUTING.md)，安全问题请使用 [私密漏洞报告](https://github.com/yuxino/tick/security/advisories/new)。

[MIT](LICENSE) © 2026 [yuxino](https://github.com/yuxino)
