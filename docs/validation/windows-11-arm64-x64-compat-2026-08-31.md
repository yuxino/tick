# Windows 11 ARM64 上的 x64 兼容层验收记录

状态：`release-verified`。验收时间为 2026-08-31（Asia/Shanghai），Guest 为 Windows 11 25H2 ARM64，Tick x64 通过 Windows x64 兼容层运行，安装范围为当前用户。本页不是实体或原生 x64 Windows 设备证据，也不是 Tick ARM64 安装包的原生交互证据。

旧的 [Windows 11 ARM64 host 准备记录](windows-11-arm64-2026-08-30.md) 只对应 v0.1.0 ARM64 候选包及 host 侧检查；其制品哈希不适用于本页。

## 精确制品

第一阶段使用行为修复提交 `b919d90b07ee250e8e72ac4f201cb18d4f1794a4` 的精确 CI x64 制品完成完整生命周期验收。该提交仍标记为 `0.1.0`，不等于 v0.1.1 发布资产。

| 阶段 | CI run | 文件 | 大小 | SHA-256 |
| --- | --- | --- | ---: | --- |
| 完整行为回归 | `33323799326` | `Tick_0.1.0_x64-setup.exe` | 5,923,535 bytes | `1a9045a9b20891e62f3abf6f009db1469135b39b39df86b00f716d276ac894cd` |
| 完整行为回归 | `33323799326` | `Tick.exe` | 16,276,480 bytes | `798335964ec01fe1564ce21980a5dd0215e7ed32f28ba458db4ec00937438cfd` |
| v0.1.1 候选复核 | `33325136154` | `Tick_0.1.1_x64-setup.exe` | 5,923,330 bytes | `b15b180c8bdec3802d9d40207c76fbad4296e2f9eae908fa53552588e2606277` |
| v0.1.1 候选复核 | `33325136154` | `Tick.exe` | 16,276,480 bytes | `4f285f86d598b4c9d1be52e657b708aafe60422dff3d9e667e82cee835eb56b4` |
| v0.1.1 tag draft 精确复核 | `33325494880` | `Tick_0.1.1_x64-setup.exe` | 5,924,410 bytes | `c5c0115326ccaad4a62b4226be2b102ec532e7fe738d64010db3515265a93418` |
| v0.1.1 公开资产重下载 | `v0.1.1` | `Tick_0.1.1_x64-setup.exe` | 5,924,410 bytes | `c5c0115326ccaad4a62b4226be2b102ec532e7fe738d64010db3515265a93418` |

第一阶段安装器是 NSIS 自解压程序；安装后的 `Tick.exe` 为 AMD64（`IMAGE_FILE_MACHINE_AMD64`）。安装目录为当前用户的 `%LOCALAPPDATA%\Tick`，安装和运行均未请求管理员权限。

v0.1.1 候选复核绑定提交 `b9269d6320609623ec1478f930a59a2ea184ade1`。重新安装后，Node.js 和已保存的 DeepSeek 配置都能恢复；再次真实输入“1分钟后提示我”可进入可编辑审阅表单，显示 60 秒重复风险，并生成通过 `TICK_EXECUTABLE --show-message` 调用 Tick 原生弹窗的脚本。候选包随后通过保存、启用、立即运行、原生弹窗、停止当前实例和删除烟雾测试。

tag workflow run `33325494880` 生成的 draft x64 安装器另行下载到共享目录并在同一 Guest 安装。该精确资产再次通过 DeepSeek 审阅、保存、启用、原生弹窗、停止、删除、托盘退出和卸载回归；卸载后程序目录不存在、Tick 系统任务数为 0，未勾选删除应用数据时设置按设计保留。发布为 [v0.1.1](https://github.com/yuxino/tick/releases/tag/v0.1.1) 并设为 Latest 后，又从公开地址重新下载 x64 安装器和 `SHA256SUMS.txt`；公开安装器与 draft 逐字节一致，文件大小和 SHA-256 均与上表及清单一致。

该 Release 恰好包含四项资产：x64 安装器、ARM64 安装器、Apple 芯片 DMG 和 `SHA256SUMS.txt`。清单自身 SHA-256 为 `c5ef6908f3802bd4697a2af51c8323141af4d281091547db8e5f2026c5a14410`；清单记录的 ARM64 安装器为 `4e8f8180303eb7d2e8a5a1ee1f51e815bb6c112de0d75bab21827e66c7bdaaaf`，DMG 为 `a40124ea293d426bbb6114480f9d0c854bca9601447ef8f719b14a368c89eef4`，x64 安装器为上表的 `c5c0115326ccaad4a62b4226be2b102ec532e7fe738d64010db3515265a93418`。这里对 ARM64 安装器和 DMG 只做发布清单与托管资产摘要核对，不把它们算作本页的 Guest 交互证据。

## Windows Guest 实测结果

| 项目 | 结果 | 现场结果 |
| --- | --- | --- |
| 安装、启动、卸载项 | 通过 | 当前用户安装，无 UAC；启动主界面正常，程序和功能中显示 Tick。 |
| Node.js 检测 | 通过 | 检测到用户已安装的 Node.js `v24.19.0` 与实际可执行文件；Tick 没有安装 Node.js，也没有修改 PATH。 |
| API Key 保存与调用 | 通过 | Key 只以掩码显示；重装后仍可读取，DeepSeek 连接测试成功。验收未记录 Key、尾号或响应正文。 |
| “1分钟后提示我” | 通过 | AI 草稿使用 Tick 内置原生提示命令，审阅后保存为每 60 秒任务；这是重复提醒，不是一次性提醒。 |
| Task Scheduler 注册 | 通过 | 创建、启用和读取均成功；注册后 URI 被规范化为带前导反斜杠的形式，Principal 被规范化为 SID，Tick 仍以当前进程 token SID 判断为同一所有者。 |
| 真实定时触发 | 通过 | 单独等待真实 60 秒边界后出现标题为 `Tick`、正文为“时间到了”的原生 Windows 弹窗；不是用“立即运行”替代。 |
| 停止、停用与日志 | 通过 | 弹窗运行期间停用任务，当前运行实例和弹窗一起停止；stdout、stderr 没有 Tick 运行错误。原生弹窗任务 stdout 为空属于预期。 |
| 编辑任务 | 通过 | 将 60 秒更新为 300 秒并保存，界面和系统任务均显示每 5 分钟。 |
| 陌生同名任务保护 | 通过 | 将同名系统任务的 action 精确篡改为 `cmd.exe` 后，刷新显示可操作的原因和下一步；启用与删除均被拒绝，Tick 没有接管或覆盖该任务。 |
| 删除 | 通过 | 外部清理精确测试任务后，Tick 显示系统任务缺失；随后删除本地记录，任务列表归零。 |
| 托盘与退出 | 通过 | 关闭窗口后托盘图标保留；托盘“显示窗口”可恢复主界面，“退出”后图标消失。 |
| 卸载与残留 | 通过 | 卸载完成后 `%LOCALAPPDATA%\Tick` 不存在、Tick 系统任务数为 0。未勾选“删除应用数据”，因此设置文件和空任务索引按卸载器选项保留，用于后续重装验证配置恢复。 |
| JSON 提取、修复、响应校验与重试 | 自动化回归通过 | Rust 单元测试覆盖代码围栏/解释文字中的 JSON、相对分钟常见字段修复、截断响应诊断、Windows 原生提醒约束和缺失 Node 的原因/下一步。此项不是额外一次 UI 故障注入证据。 |

## 所有权与 Principal 断言

CI 在真实 Windows Task Scheduler 中注册临时任务，读取注册后的规范化 Principal，将连接账户名与注册值都解析为 SID，再使用 `EqualSid` 断言与当前进程 token SID 等价。该断言覆盖了本机账户名与 Task Scheduler 规范化 Principal 的等价关系；AzureAD、Microsoft 账户和跨域名称变体仍未单独做原生设备覆盖。

## 证据边界

- 本页的完整交互证据来自 Windows 11 ARM64 上的 x64 兼容层；Windows x64 实体设备和 Tick ARM64 原生交互仍未验证。
- Windows ARM64 安装包目前只有 CI 构建、PE 架构和安装器内容校验，不能据此推断原生安装或交互通过。
- Windows 包没有 Authenticode 签名，系统可能显示未知发布者或来源提示；它不是商店级签名分发。
- JavaScript 任务依赖用户自行安装 Node.js。Tick 只检测并报告，不下载、不安装，也不修改 PATH。
- Windows API Key 保存在当前用户配置目录，依赖该目录的访问控制，不属于 Credential Manager 或加密保险库。
- v0.1.1 tag draft 已绑定源码 SHA、CI run、文件大小和 SHA-256，并完成安装、启动、配置恢复及任务关键烟雾测试；公开 Release 的 x64 资产已重新下载，且与 draft 及发布清单逐字节一致。
