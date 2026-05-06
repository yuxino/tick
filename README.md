# Tick

Tick 是一个用 Tauri + React + TypeScript 写的桌面应用，用来管理 macOS 用户级 `launchd` 定时任务。

## 功能

- 新建、编辑、删除、启用和停用由 Tick 管理的 LaunchAgent。
- 配置按月、日、时、分、秒运行的固定时间计划。
- 配置每隔 N 秒执行一次的间隔计划。
- 运行内联 shell 脚本、可执行脚本路径，或通过 Node.js 等解释器执行命令。
- 使用语法高亮编辑内联脚本。
- 查看 stdout/stderr 日志、清空日志，并支持自动刷新。
- 预览每个任务生成的 plist 配置。

## launchd 行为

Tick 只管理用户级 LaunchAgent，目录是：

```text
~/Library/LaunchAgents
```

Tick 生成的 label 使用这个前缀：

```text
com.gavin.tick.
```

日志和托管的内联脚本会放在应用数据目录下的 `tick` 目录里。

`launchd` 的固定时间计划支持月、日、时、分，但不直接支持秒。Tick 会生成一个 wrapper 脚本，先 sleep 指定秒数再执行命令。间隔计划使用原生 `StartInterval`。

LaunchAgent 不会加载交互式 shell 的 profile。解释器、脚本和工作目录都建议使用绝对路径，例如 `/opt/homebrew/bin/node`。

## 开发

安装依赖：

```bash
npm install
```

运行 Web UI：

```bash
npm run dev
```

运行桌面应用：

```bash
npm run tauri dev
```

构建前端：

```bash
npm run build
```

检查并测试 Rust 后端：

```bash
cd src-tauri
cargo check
cargo test
```
