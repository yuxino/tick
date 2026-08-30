# Windows Task Scheduler 支持设计

## 目标与边界

Tick 在 macOS 继续管理当前用户的 `LaunchAgent`，在 Windows 11 使用当前用户的 Task Scheduler。Windows 后端不请求管理员权限，不创建系统级任务，不枚举或修改非 Tick 任务。所有修改操作都从 Tick 自有 registry 中读取任务，再校验任务 ID、私有命名空间和 Task Scheduler XML 中的所有权标记；同名但不满足标记的任务视为冲突并拒绝覆盖。

Windows Task Scheduler 原生重复间隔为 60 秒至 31 天。Windows UI 与 Rust 校验同时采用这个范围并直接报错，不静默取整；macOS 保持原有最小 1 秒行为。日、月、年日历触发器保留秒级开始时间。

## 架构与数据流

前后端模型改为平台无关的 scheduled job、definition path 和 scheduler capabilities。macOS adapter 继续生成 plist 并调用 `launchctl`；Windows adapter 通过 Task Scheduler COM API 注册 `InteractiveToken`、`LeastPrivilege` 的当前用户任务。Windows task name 只由严格校验的 `job-<20 位十进制随机数>` ID 派生；随机数来自 UUID v4，不依赖可能冲突的本机时间戳。

Task Scheduler action 只包含当前 Tick 可执行文件、`--run-scheduled-job` 和任务 ID。用户填写的解释器、脚本路径、参数、工作目录和环境变量不会拼入 XML action 或 shell；Tick runner 从自有 registry 重新派生脚本与日志路径。Windows runner 使用 `CreateProcessW(CREATE_SUSPENDED | CREATE_NO_WINDOW)` 创建子进程，先加入 kill-on-close Job Object，再恢复线程，以消除子进程在纳入停止边界前逃逸的竞态；命令行由 Windows argv 规则逐参数转义，不经过 shell，并在保存与执行时检查 Win32 UTF-16 长度上限。环境块使用 Windows Unicode ordinal 规则去重和排序。

间隔任务的首次边界设为“保存时间 + 间隔”，并关闭 `StartWhenAvailable`，避免保存后立即补跑或错过边界后集中补跑。日历任务从用户选择的下一次本地时间开始。

## 生命周期与错误处理

- 保存：先校验输入和所有权，再生成定义并创建或更新 Tick 自有任务；disabled 任务也注册为 disabled。registry 原子写入失败时回滚平台任务。
- 启用：校验 registry、task name 与 XML 所有权后启用。
- 立即运行：由 Task Scheduler 启动 runner，日志仍落在 Tick 自有目录。
- 停用：先禁用未来触发，再停止当前实例；连续确认 Task Scheduler 明确报告 `DISABLED` 且实例数归零后才成功。`UNKNOWN`、`READY` 或读取失败都不会放行。任何所有权不符或系统调用失败均向 UI 报错。
- 删除：先校验所有权，禁用并停止实例、连续确认上述 settled 状态并精确删除该任务，再删除由 ID 派生的定义、脚本、日志并更新 registry。
- 状态：分别返回 enabled、disabled、missing 或 error；UI 不再把 enabled 写成“正在运行”。

registry 中持久化的 label 和路径不再直接作为读写目标。每次读取都校验 ID/label 并从 Tick 数据目录重新派生 definition、stdout、stderr 与 materialized script 路径，防止被篡改的 registry 指向任意文件或系统任务。跨进程调度操作锁覆盖平台修改、registry 更新和回滚全程，registry 自身的读改写再由独立锁文件串行化，并通过同目录临时文件与原子替换持久化。runner 只在读取任务快照时取得共享操作锁，随后释放，避免阻塞停用。

## 验证

自动化测试覆盖身份/路径派生、XML 转义和所有权标记、Windows 间隔边界、Windows 参数拆分、环境变量校验以及 macOS plist 回归。完成 macOS canonical checks 后，在指定 Windows 11 25H2 ARM64 UTM 环境构建 NSIS、当前用户安装并启动，使用含空格、中文与 shell 元字符的安全测试目录验证新建、启用、运行、停止、日志、托盘、退出和卸载。验收结束只删除本次创建的测试任务，并保存不含私人内容的截图和命令证据。
