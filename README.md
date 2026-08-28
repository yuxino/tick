<div align="center">
  <img src="public/tick-icon.png" width="96" alt="Tick">
  <h1>Tick</h1>
  <p>把 macOS LaunchAgent 变成看得懂、改得动的定时任务。</p>
  <p>
    <a href="https://github.com/yuxino/tick/releases/latest">下载</a>
    · <a href="#开发">从源码运行</a>
    · <a href="https://github.com/yuxino/tick/issues">Issue</a>
  </p>
</div>

![Tick 主界面](docs/images/tick-overview.png)

## 能做什么

- 创建、编辑、暂停、立即运行和删除任务
- 按日期、时间或固定间隔调度
- 直接编写 Node.js，或运行已有 `.js` 文件
- 保存前试跑脚本，查看 stdout、stderr 与 plist
- 在月历中检查固定时间任务
- 可选用 DeepSeek 从一句话生成可编辑草稿

Tick 只管理自己创建的用户级 LaunchAgent，不会修改系统 daemon。AI 生成的内容不会直接执行，保存前仍可检查和修改。

## 本地数据

| 内容 | 位置 |
| --- | --- |
| LaunchAgent | `~/Library/LaunchAgents/com.gavin.tick.*.plist` |
| 脚本和日志 | `~/Library/Application Support/tick/` |
| DeepSeek API Key | `~/Library/Application Support/com.gavin.tick/settings.json` |

DeepSeek 完全可选。API Key 文件权限为 `0600`，但仍是本机明文文件，可随时在设置中删除；不用 DeepSeek 时，Tick 不需要任何 API Key。

## 开发

需要 Node.js、Rust 和 macOS。

```bash
npm install
npm run tauri dev
```

检查代码：

```bash
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
```

构建应用：

```bash
npm run tauri build
```

Tick 目前只支持 macOS。贡献说明见 [CONTRIBUTING.md](CONTRIBUTING.md)，安全问题请使用 [私密漏洞报告](https://github.com/yuxino/tick/security/advisories/new)。

[MIT](LICENSE) © 2026 [yuxino](https://github.com/yuxino)
