# DeepSeek 草稿恢复设计

## 目标与边界

修复 Windows 中输入“1分钟后提示我”后，DeepSeek 草稿因 JSON 形态与 Tick 的严格 Rust 模型不完全一致而直接失败的问题。Tick 仍只支持日历和固定间隔任务；相对时间会被转换为固定间隔，并明确提示它会重复运行。此次不新增一次性调度模式；Windows 通知通过 Tick 自带的原生提示窗口实现，不把日志伪装成通知，也不依赖 PowerShell。

## 数据流与兼容策略

自动化请求启用 DeepSeek 的 JSON Output。响应先检查空内容与 token 截断，再从代码围栏或说明文字中提取第一个完整 JSON 对象。兼容层只补全 Tick 已有模型中的安全结构：支持 `task`/`job`、`title`/`name`、`script`/`inlineScript` 等常见别名，补齐当前模式未使用但 Rust 模型要求存在的 `calendar`、`interval` 和 execution 字段，并把 `1分钟后`、`每1小时`等阿拉伯数字时间表达转换为秒。最终仍必须通过现有任务校验和原生能力校验，不能绕过间隔范围、脚本类型或平台安全限制。

首次响应不能解析或校验失败时，Tick 把具体失败原因连同原响应交给 DeepSeek 自动修复一次。第二次仍失败时，界面展示原因，并提供“改为手动填写”按钮；不再只要求用户换一种说法。

## Windows API Key

API Key 继续保存在当前用户的 `%APPDATA%\com.gavin.tick\settings.json`，调用路径与 macOS 一致。原实现用普通 rename 覆盖设置文件，在 Windows 上更新或删除已有 Key 可能失败。设置写入改用与任务索引相同的 `MoveFileExW(REPLACE_EXISTING | WRITE_THROUGH)` 原子替换，并保留临时文件清理与回归测试。

## 验证

Rust 回归覆盖围栏/说明文字提取、截断诊断、缺字段与别名修复、“1分钟后”的 60 秒解析、重复运行风险提示、Windows 原生提示调用校验、token 截断拒绝，以及已有设置文件的原子替换。完整 `npm run check` 覆盖格式、Clippy、Rust 测试、路径隐私和前端构建；Windows x64 CI 再运行同一门禁、生成 NSIS 安装包并检查 PE 架构与嵌入路径。原生安装、真实 DeepSeek Key 保存/替换/调用及界面点击由用户在 Windows 中手动复测。
