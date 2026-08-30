# 参与 Tick

感谢你愿意帮助改进 Tick。

## 开始之前

- macOS 只管理由 Tick 创建、且 label 以 `com.gavin.tick.` 开头的当前用户 LaunchAgent；Windows 只管理带 Tick 所有权标记、名称以 `Tick.job-` 开头的当前用户 Task Scheduler 任务。
- 不要扩大到系统级 daemon、系统任务或管理员权限；若确有必要，请先在 Issue 中说明动机、权限边界与迁移方案。
- Windows 固定间隔必须保持在 60 秒至 31 天；不要静默取整。macOS 最短间隔为 1 秒。
- 行为变化应同时更新 README 或相关界面说明。

## 本地开发

以下流程已在 Apple 芯片（M 系列、arm64）Mac 上使用。Windows 需要 Microsoft C++ Build Tools 和 WebView2；Windows 11 ARM64 的原生验收仍在进行，Intel Mac 尚未验证。

```bash
npm install
npm run tauri dev
```

提交前请运行：

```bash
npm run check
```

Windows release/installer builds must use:

```bash
npm run build:windows -- --bundles nsis -- --locked
```

This keeps Rust panic locations for diagnostics while remapping the build machine's home, Cargo cache, and checkout prefixes out of shipped binaries. CI extracts the NSIS payload and rejects ASCII or UTF-16 user-home paths in both copies of `tick.exe`.

## 提交 Pull Request

1. 先搜索已有 Issue 和 Pull Request，避免重复工作。
2. 一个 PR 尽量只解决一个问题。
3. 说明改了什么、为什么改，以及如何验证。
4. UI 变化请附上截图。
5. 不要提交真实日志、用户名路径、API Key、构建缓存或个人 LaunchAgent / Task Scheduler 配置。

小修复可以直接提交 PR；较大的行为或架构调整，建议先开 Issue 讨论。
