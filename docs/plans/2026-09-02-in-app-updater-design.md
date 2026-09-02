# Tick 应用内更新设计

## 决策

采用 Tauri 2 官方 updater、process 和 opener 插件。updater 负责检查、下载、强制验证签名和安装；process 只在 macOS/Linux 用户明确点击后重启；opener 只允许打开 Tick 的 GitHub Releases 页面，且仅作为错误恢复入口。

未采用直接请求 GitHub API 再跳浏览器的方案，因为它不能完成签名验证与应用内安装。也暂不增加自建更新服务；GitHub Release 的静态 `latest.json` 已满足当前三种发行架构，维护面最小。

## 状态与数据流

设置页提供 `idle → checking → current/available → downloading → installing → ready` 状态机。检查始终由用户触发；发现更新后展示版本号、发布日期与纯文本 release notes，再由用户点击下载。下载进度直接消费 updater 的 `Started`、`Progress`、`Finished` 事件：有总字节数时计算真实百分比，没有时显示不确定进度且不伪造数字。

下载完成后立即调用 updater 的 `install()`。该调用使用应用内置公钥强制验证签名，失败即停止安装并进入可重试错误态。macOS/Linux 安装完成后等待用户点击“重启并完成更新”；Windows 安装器启动时应用会按框架限制退出，因此界面只说明即将交给安装器，不承诺可延迟重启。

## 发布与安全

应用只包含 updater 公钥和固定 HTTPS feed。加密私钥只保存在本机受保护目录和 GitHub Actions Secret，密码同时保存在系统钥匙串与 GitHub Secret。发布任务为 macOS arm64、Windows x64、Windows arm64 生成 updater 资产和 `.sig`，再生成并验证 `latest.json` 的版本、平台、URL、签名字段和资产唯一性。

v0.1.4 是 bootstrap 版本：v0.1.3 及更旧版本没有 updater，必须手动安装一次 v0.1.4；后续版本才可从应用内升级。

## 验证

单元测试覆盖检查结果、已知与未知总大小、重复事件、网络/签名/取消错误文案和重试所需状态。发布脚本用受控 fixture 验证正常 feed、签名字段损坏、平台缺失、URL 或版本不一致。发布后核验公开 `latest.json`、资产 SHA-256、`.sig` 可访问性，并在 exact public artifact 中至少验证启动和“已是最新版”路径。真实跨版本升级要等下一版本才具备公开源版本，因此必须单独报告。
