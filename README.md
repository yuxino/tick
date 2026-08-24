<div align="center">
  <img src="src-tauri/icons/128x128@2x.png" width="92" alt="Tick">
  <h1>Tick</h1>
  <p>一个简单的 macOS LaunchAgent 定时任务工具。</p>
  <p>
    <a href="https://github.com/yuxino/tick/releases/latest">下载</a>
    · <a href="#开发">从源码运行</a>
    · <a href="https://github.com/yuxino/tick/issues">Issue</a>
  </p>
</div>

![Tick 主界面](docs/images/tick-overview.jpg)

## 功能

- 创建、编辑、启停和立即运行定时任务
- 按日期、时间或固定间隔运行
- 编写 Node.js，或运行现有 `.js` 文件
- 保存前调试脚本
- 查看日程、stdout、stderr 和 plist
- 可选使用 DeepSeek 从一句话生成任务草稿

Tick 只管理自己创建的用户级 LaunchAgent，不会修改系统 daemon。

## 本地数据

| 内容 | 位置 |
| --- | --- |
| LaunchAgent | `~/Library/LaunchAgents/com.gavin.tick.*.plist` |
| 脚本和日志 | `~/Library/Application Support/tick/` |
| DeepSeek API Key | `~/Library/Application Support/com.gavin.tick/settings.json` |

DeepSeek 完全可选。API Key 文件权限为 `0600`，但仍是本机明文文件，可随时在设置中删除。

## 开发

需要 Node.js、Rust 和 macOS。

```bash
npm install
npm run tauri dev
```

检查：

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
