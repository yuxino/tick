# 安全政策

Tick 可以创建和运行本地脚本，也会写入用户的 `~/Library/LaunchAgents`。如果你发现命令注入、越权文件访问、意外管理非 Tick 任务，或其他可能危及用户系统的问题，请不要先公开 Issue。

DeepSeek API Key 由用户在应用内提供，保存在应用设置目录的 `settings.json` 中。目录权限为 `0700`，文件权限为 `0600`；这能阻止其他本机用户直接读取，但不属于加密存储。Tick 不会把密钥写入源码或日志，Rust 后端只向前端返回配置状态和脱敏提示。

请通过 GitHub 的私密漏洞报告功能联系维护者：

<https://github.com/yuxino/tick/security/advisories/new>

报告中请包含受影响版本、复现步骤、可能影响，以及你建议的缓解方式。维护者会在确认问题后协调修复与公开披露。
