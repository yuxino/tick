# 参与 Tick

感谢你愿意帮助改进 Tick。

## 开始之前

- Tick 只管理由自己创建、且 label 以 `com.gavin.tick.` 开头的用户级 LaunchAgent。
- 请不要扩大到系统级 daemon，除非先在 Issue 中说明动机、权限边界与迁移方案。
- 行为变化应同时更新 README 或相关界面说明。

## 本地开发

```bash
npm install
npm run tauri dev
```

提交前请运行：

```bash
npm run build
cd src-tauri
cargo fmt --check
cargo test
```

## 提交 Pull Request

1. 先搜索已有 Issue 和 Pull Request，避免重复工作。
2. 一个 PR 尽量只解决一个问题。
3. 说明改了什么、为什么改，以及如何验证。
4. UI 变化请附上截图。
5. 不要提交真实日志、用户名路径、API Key、构建缓存或个人 LaunchAgent 配置。

小修复可以直接提交 PR；较大的行为或架构调整，建议先开 Issue 讨论。
