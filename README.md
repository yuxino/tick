<div align="center">
  <img src="public/tick-icon.png" width="96" alt="Tick">
  <h1>Tick</h1>
  <p>把系统定时任务变成看得懂、改得动的自动化。</p>
  <p>
    <a href="https://github.com/yuxino/tick/releases/latest">下载最新版</a>
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

执行 JavaScript 任务需要用户自行安装 Node.js。Windows 版会检查 PATH、Node.js 官方注册表、`NVM_SYMLINK` 指向和常见安装目录，并在设置中显示实际验证过的版本与路径；Tick 不内置、下载或安装 Node.js，也不会修改 PATH。安装后可点击“重新检测”，仍未识别时请完全退出 Tick 后重新打开。DeepSeek 在 Windows 生成提醒或通知类草稿时会调用 Tick 自带的原生提示窗口，stdout 和 stderr 只用于日志，不需要额外安装弹窗程序。

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

需要 Node.js 22 和 Rust。macOS 还需要 Xcode Command Line Tools；Windows 需要 Microsoft C++ Build Tools 和 WebView2。当前 CI 在 Apple 芯片（M 系列、arm64）macOS、Windows x64 和 Windows ARM64 上运行检查并构建对应安装包；CI 与安装包构建不等于原生安装和交互验收，Intel Mac 尚未验证。

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

[v0.1.1](https://github.com/yuxino/tick/releases/tag/v0.1.1) 提供 Apple 芯片 macOS DMG，以及 Windows x64、Windows ARM64 的当前用户 NSIS 安装包。Windows x64 发布资产已在 Windows 11 ARM64 的 x64 兼容环境完成[安装和核心交互验收](docs/validation/windows-11-arm64-x64-compat-2026-08-31.md)，公开后重新下载并核对了 SHA-256；Windows ARM64 安装包目前只有 CI 构建与架构校验证据。Intel Mac 尚未验证。macOS 包没有 Developer ID 签名或 Apple 公证，Windows 包没有 Authenticode 签名，系统可能显示来源或发布者提示；这些是可手动确认安装的开发者分发包，不代表商店级签名。贡献说明见 [CONTRIBUTING.md](CONTRIBUTING.md)，安全问题请使用 [私密漏洞报告](https://github.com/yuxino/tick/security/advisories/new)。

[MIT](LICENSE) © 2026 [yuxino](https://github.com/yuxino)
