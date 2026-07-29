<div align="center">
  <img src="src/assets/tick-mascot.png" width="92" alt="Tick">
  <h1>Tick</h1>
  <p>一个用来折腾 macOS LaunchAgent 的小工具。</p>
  <p>
    <a href="https://github.com/yuxino/tick/releases/latest">下载</a>
    · <a href="#自己跑起来">从源码运行</a>
    · <a href="https://github.com/yuxino/tick/issues">提 Issue</a>
  </p>
</div>

![Tick 主界面](docs/images/tick-overview.jpg)

## 为什么会有 Tick

我做 Tick，主要是想把 LaunchAgent 搞明白。

一开始只是嫌 plist 难写、`launchctl` 难记，出了问题还得四处找日志。后来干脆把常用操作做成了界面：任务什么时候跑、实际写出了什么 plist、stdout 和 stderr 在哪，都可以直接看到。

所以它不是 Alfred、Raycast 或专业自动化工具的替代品。更准确地说，Tick 是我学习 LaunchAgent 时顺手做出来的实验品，现在也确实能拿来管理一些简单的定时任务。

<table>
  <tr>
    <td width="50%"><img src="docs/images/tick-create-job.jpg" alt="创建任务"></td>
    <td width="50%"><img src="docs/images/tick-logs.jpg" alt="查看日志"></td>
  </tr>
  <tr>
    <td align="center">创建任务时顺便看看 plist</td>
    <td align="center">任务不听话就翻 stdout / stderr</td>
  </tr>
</table>

## 现在能做什么

- 创建、编辑、启停和删除 Tick 自己管理的 LaunchAgent
- 按日期、时间或固定间隔运行任务
- 跑 shell、脚本文件和 Node.js 脚本
- 手动运行一次，确认脚本到底会不会工作
- 看日志、看 plist，也可以打开日程视图找下一次运行时间
- 用一句话生成任务名称、运行时间、脚本和风险提示，再检查、试运行和保存

Tick 只碰自己创建的用户级任务，不会去改系统 daemon。

| Tick 写入的东西 | 位置 |
| --- | --- |
| LaunchAgent | `~/Library/LaunchAgents/com.gavin.tick.*.plist` |
| 日志和内联脚本 | `~/Library/Application Support/tick/` |
| DeepSeek API Key | 应用设置目录中的 `settings.json`（仅当前用户可读写） |

固定时间使用 `StartCalendarInterval`，固定间隔使用 `StartInterval`。LaunchAgent 不会加载交互式 shell 的 profile，所以解释器、脚本和工作目录最好写绝对路径。这个坑我已经踩过了。

## DeepSeek

DeepSeek 是可选的。点右上角设置，填一次自己的 API Key，之后 Tick 会自动读取。

AI 入口不是单独写一段代码，而是根据一句话搭好整个任务草稿：名称、说明、运行计划、脚本和风险提示都会一起生成。Tick 不会直接启用它，先检查和试运行，觉得没问题再保存。

Key 存在应用设置目录中，文件权限限制为当前用户读写。它不会进源码或日志，但仍然是本机明文文件；如果电脑账户本身不可信，不要使用这个功能。不想用了，直接在设置里删掉。

## 自己跑起来

需要 Node.js、Rust，以及能构建 Tauri 2 应用的 macOS 环境。

```bash
git clone https://github.com/yuxino/tick.git
cd tick
npm install
npm run tauri dev
```

跑一下检查：

```bash
npm run build
cd src-tauri
cargo test
```

构建 `.app`：

```bash
npm run tauri build
```

## 还在长

Tick 目前只支持 macOS，也还是个很早期的小项目。界面、任务类型和错误提示都可能继续改。

想一起折腾的话，Issue 和 PR 都欢迎。开发约定在 [CONTRIBUTING.md](CONTRIBUTING.md)，安全问题请走 [私密漏洞报告](https://github.com/yuxino/tick/security/advisories/new)。

角色图标是为 Tick 画的原创时钟精灵，不属于现有角色系列。

[MIT](LICENSE) © 2026 [yuxino](https://github.com/yuxino)
