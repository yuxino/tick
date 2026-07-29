<div align="center">
  <img src="src/assets/tick-mascot.png" width="92" alt="Tick">
  <h1>Tick</h1>
  <p>一个给 macOS 用的定时任务应用，也支持用 DeepSeek 从一句话起草任务。</p>
  <p>
    <a href="https://github.com/yuxino/tick/releases/latest">下载</a>
    · <a href="#从源码运行">从源码运行</a>
    · <a href="https://github.com/yuxino/tick/issues">提 Issue</a>
  </p>
</div>

![Tick 主界面](docs/images/tick-overview.jpg)

## 不用再手写 plist

Tick 把 macOS 自带的 LaunchAgent 做成了一个看得见的任务列表。

新建任务时，填好要运行的脚本和时间就行。Tick 会生成 plist、加载任务，并把日志收在固定的位置。之后要临时运行一次、改时间、停用任务或者查错，也都在同一个窗口里完成。

每天跑一段 Node.js、每周调用一次备份脚本、隔一段时间整理文件——这类本地任务都可以交给 Tick。

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

## 用 Tick 做什么

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

## 也可以直接说你想做什么

新建任务时，可以手动填写，也可以先用一句话描述。Tick 会请 DeepSeek 起草名称、运行计划和脚本，遇到值得留意的操作也会一起标出来。生成结果只是一份草稿，不会跳过确认直接启用。

这部分完全可选。不配置 DeepSeek，其他功能照常使用。

API Key 只需要在设置里填一次。Tick 会把它存在当前用户的应用设置目录，文件权限限制为当前用户读写。Key 不会进入项目源码或任务日志，但它仍然是本机明文文件；不想保留时，可以随时从设置里删除。

## 它是怎么来的

我经常用 LaunchAgent 跑一些定时脚本，但一直觉得它缺一个简单的界面。每次手写 plist、查 `launchctl`、再去翻日志，事情不难，过程却很碎。

做 Tick 时正好赶上 DeepSeek V4 发布，自然语言创建任务也就成了应用的一部分。应用的目标一直很具体：把任务建好，让它准时运行，出了问题也知道去哪里看。

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
