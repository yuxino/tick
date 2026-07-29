<div align="center">
  <img src="src/assets/tick-mascot.png" width="92" alt="Tick">
  <h1>Tick</h1>
  <p>在窗口里管好自己的 macOS LaunchAgent。</p>
  <p>
    <a href="https://github.com/yuxino/tick/releases/latest">下载</a>
    · <a href="#从源码运行">从源码运行</a>
    · <a href="https://github.com/yuxino/tick/issues">提 Issue</a>
  </p>
</div>

![Tick 主界面](docs/images/tick-overview.jpg)

## 这是个什么东西

我写 Tick，最初只是想弄明白 macOS 的 LaunchAgent。

plist 不难，但手写很烦；`launchctl` 的命令也总记不住。任务没跑起来时，还要再去找 stdout、stderr 和加载状态。Tick 把这些东西放进了一个窗口：什么时候运行、实际生成了什么配置、日志写到哪，都能直接看。

项目开工时正好赶上 DeepSeek V4 发布，我也想找个真实场景试试它。于是 Tick 多了一个可选的 AI 入口：说一句想自动完成的事，它会起草运行时间和脚本。草稿不会直接启用，还是要自己看过、试过，再保存。

这不是 Alfred 或 Raycast 的替代品，也没打算包办所有自动化。它就是一个小工具，顺便记录我学习 LaunchAgent 和体验 DeepSeek V4 的过程。

<table>
  <tr>
    <td width="50%"><img src="docs/images/tick-create-job.jpg" alt="新建任务"></td>
    <td width="50%"><img src="docs/images/tick-logs.jpg" alt="查看日志"></td>
  </tr>
  <tr>
    <td align="center">说一句想做什么，也可以直接手动填写</td>
    <td align="center">任务没动静时，先看看 stdout 和 stderr</td>
  </tr>
</table>

## 能做的事

- 创建、编辑、启停和删除 Tick 自己管理的 LaunchAgent
- 按固定日期、时间或间隔运行任务
- 执行 Node.js、脚本文件和普通命令
- 保存前先调试运行一次
- 查看 plist、stdout、stderr 和接下来的运行日程
- 用 DeepSeek 起草一个完整任务

Tick 只管理它自己创建的用户级任务，不碰系统 daemon。

| 文件 | 位置 |
| --- | --- |
| LaunchAgent | `~/Library/LaunchAgents/com.gavin.tick.*.plist` |
| 日志和内联脚本 | `~/Library/Application Support/tick/` |
| DeepSeek API Key | 应用设置目录中的 `settings.json` |

固定时间对应 `StartCalendarInterval`，固定间隔对应 `StartInterval`。还有一个容易踩的坑：LaunchAgent 不会加载交互式 shell 的 profile，所以解释器、脚本和工作目录最好写绝对路径。

## DeepSeek 是可选的

不用 AI，Tick 也能正常创建和管理任务。

如果要用，在右上角设置里填一次 API Key。Tick 会把 Key 存在当前用户的应用设置目录，文件权限限制为当前用户读写。它不会进入项目源码或任务日志，但仍然是本机明文文件。不想留着时，可以随时从设置里删除。

AI 生成的是一份待检查的草稿，包括名称、说明、运行计划、脚本和风险提示。Tick 不会绕过确认直接把它挂进 LaunchAgent。

## 从源码运行

需要 Node.js、Rust，以及能构建 Tauri 2 应用的 macOS 环境。

```bash
git clone https://github.com/yuxino/tick.git
cd tick
npm install
npm run tauri dev
```

检查前端和 Rust：

```bash
npm run build
cd src-tauri
cargo test
```

构建 `.app`：

```bash
npm run tauri build
```

## 项目状态

Tick 目前只支持 macOS，功能和界面还会继续调整。Issue 和 PR 都欢迎，开发约定见 [CONTRIBUTING.md](CONTRIBUTING.md)，安全问题请用 [GitHub 的私密漏洞报告](https://github.com/yuxino/tick/security/advisories/new)。

图标是为 Tick 画的原创时钟精灵，不属于现有角色系列。

[MIT](LICENSE) © 2026 [yuxino](https://github.com/yuxino)
