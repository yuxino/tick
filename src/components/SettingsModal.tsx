import {
  CheckCircleFilled,
  CloudDownloadOutlined,
  CodeOutlined,
  DeleteOutlined,
  LinkOutlined,
  KeyOutlined,
  ReloadOutlined,
  WarningFilled,
} from "@ant-design/icons";
import { Alert, Button, Input, Modal, Popconfirm, Progress, Space, Tag, Typography, message } from "antd";
import { useCallback, useEffect, useReducer, useRef, useState } from "react";
import {
  deleteDeepSeekApiKey,
  getDeepSeekConfigStatus,
  saveDeepSeekApiKey,
  testDeepSeekConnection,
  type DeepSeekConfigStatus,
} from "../services/deepseek";
import { friendlyError } from "../utils/errors";
import type { NodeRuntimeStatus } from "../types/scheduler";
import {
  checkForAppUpdate,
  currentAppVersion,
  describeUpdateError,
  downloadPercent,
  initialUpdateState,
  isWindowsRuntime,
  openTickReleases,
  relaunchTick,
  updateViewReducer,
  type AppUpdate,
} from "../services/appUpdater";

interface SettingsModalProps {
  open: boolean;
  nodeRuntime?: NodeRuntimeStatus;
  checkingNode: boolean;
  onRecheckNode: () => Promise<void>;
  onClose: () => void;
}

export function SettingsModal({ open, nodeRuntime, checkingNode, onRecheckNode, onClose }: SettingsModalProps) {
  const [status, setStatus] = useState<DeepSeekConfigStatus>({ configured: false });
  const [apiKey, setApiKey] = useState("");
  const [loading, setLoading] = useState(false);
  const [testing, setTesting] = useState(false);
  const [updateState, dispatchUpdate] = useReducer(updateViewReducer, initialUpdateState);
  const updateRef = useRef<AppUpdate | undefined>(undefined);
  const updateActionRef = useRef(false);
  const windows = isWindowsRuntime();
  const updateBusy = ["checking", "downloading", "installing"].includes(updateState.phase);
  const percent = downloadPercent(updateState.progress);

  const loadStatus = useCallback(async () => {
    try {
      setStatus(await getDeepSeekConfigStatus());
    } catch (error) {
      message.error(friendlyError(error));
    }
  }, []);

  useEffect(() => {
    if (open) {
      setApiKey("");
      loadStatus();
      if (!updateState.currentVersion) {
        currentAppVersion()
          .then((version) => dispatchUpdate({ type: "version", version }))
          .catch(() => undefined);
      }
    }
  }, [loadStatus, open, updateState.currentVersion]);

  useEffect(() => () => {
    void updateRef.current?.close();
  }, []);

  async function releaseUpdateResource() {
    const current = updateRef.current;
    updateRef.current = undefined;
    if (current) {
      await current.close().catch(() => undefined);
    }
  }

  async function checkUpdate() {
    if (updateBusy || updateActionRef.current) return;
    updateActionRef.current = true;
    dispatchUpdate({ type: "checking" });
    try {
      await releaseUpdateResource();
      const version = updateState.currentVersion ?? await currentAppVersion();
      dispatchUpdate({ type: "version", version });
      const update = await checkForAppUpdate();
      if (!update) {
        dispatchUpdate({ type: "current" });
        return;
      }

      updateRef.current = update;
      dispatchUpdate({
        type: "available",
        update: {
          version: update.version,
          currentVersion: update.currentVersion,
          body: update.body,
          date: update.date,
        },
      });
    } catch (error) {
      dispatchUpdate({ type: "error", message: describeUpdateError(error) });
    } finally {
      updateActionRef.current = false;
    }
  }

  async function downloadAndInstallUpdate() {
    const update = updateRef.current;
    if (!update || updateBusy || updateActionRef.current) return;

    updateActionRef.current = true;
    dispatchUpdate({ type: "download-started" });
    try {
      await update.download(
        (event) => dispatchUpdate({ type: "download-event", event }),
        { timeout: 5 * 60_000 },
      );
      dispatchUpdate({ type: "installing" });
      await update.install();
      dispatchUpdate({ type: windows ? "windows-installer" : "ready" });
    } catch (error) {
      dispatchUpdate({ type: "error", message: describeUpdateError(error) });
    } finally {
      updateActionRef.current = false;
    }
  }

  async function restartAfterUpdate() {
    if (updateBusy || updateActionRef.current) return;
    updateActionRef.current = true;
    try {
      await relaunchTick();
    } catch (error) {
      dispatchUpdate({ type: "error", message: describeUpdateError(error) });
    } finally {
      updateActionRef.current = false;
    }
  }

  async function openReleasesRecovery() {
    try {
      await openTickReleases();
    } catch (error) {
      message.error(friendlyError(error));
    }
  }

  async function saveKey() {
    if (!apiKey.trim()) {
      message.warning("请先输入 DeepSeek API Key");
      return;
    }
    setLoading(true);
    try {
      const nextStatus = await saveDeepSeekApiKey(apiKey);
      setStatus(nextStatus);
      setApiKey("");
      message.success("已保存，之后会自动读取");
    } catch (error) {
      message.error(friendlyError(error));
    } finally {
      setLoading(false);
    }
  }

  async function testConnection() {
    setTesting(true);
    try {
      await testDeepSeekConnection();
      message.success("DeepSeek 连接正常");
    } catch (error) {
      message.error(friendlyError(error));
    } finally {
      setTesting(false);
    }
  }

  async function deleteKey() {
    setLoading(true);
    try {
      await deleteDeepSeekApiKey();
      setStatus({ configured: false });
      setApiKey("");
      message.success("API Key 已从本机设置中删除");
    } catch (error) {
      message.error(friendlyError(error));
    } finally {
      setLoading(false);
    }
  }

  return (
    <Modal
      open={open}
      title="设置"
      width={560}
      className="settings-modal"
      closable={!updateBusy}
      keyboard={!updateBusy}
      maskClosable={false}
      footer={<Button disabled={updateBusy} onClick={onClose}>完成</Button>}
      onCancel={() => {
        if (!updateBusy) onClose();
      }}
      destroyOnHidden
    >
      <div className="settings-stack">
        <div className="settings-section">
          <div className="settings-heading">
            <div>
              <Typography.Title level={5}>Node.js</Typography.Title>
              <Typography.Text type="secondary">用于执行 JavaScript 定时任务</Typography.Text>
            </div>
            {nodeRuntime?.available ? (
              <Tag color="success" icon={<CheckCircleFilled />}>
                {nodeRuntime.version ?? "可用"}
              </Tag>
            ) : (
              <Tag color="warning" icon={<WarningFilled />}>
                {nodeRuntime ? "未检测到" : "检测中"}
              </Tag>
            )}
          </div>

          {nodeRuntime?.available ? (
            <Typography.Paragraph type="secondary" className="settings-storage-note">
              Tick 已验证这份 Node.js 可运行。任务解释器：
              <Typography.Text code className="settings-runtime-path">
                {nodeRuntime.executablePath ?? "系统默认 Node.js"}
              </Typography.Text>
            </Typography.Paragraph>
          ) : (
            <Typography.Paragraph type="secondary" className="settings-storage-note">
              {nodeRuntime?.reason ?? "正在检测 Node.js。"} 请自行安装后重新检测；Tick 不会自动安装或修改 PATH。若刚安装仍未识别，请完全退出 Tick 后重新打开。
            </Typography.Paragraph>
          )}

          <Button icon={nodeRuntime?.available ? <ReloadOutlined /> : <CodeOutlined />} loading={checkingNode} onClick={onRecheckNode}>
            重新检测
          </Button>
        </div>

        <div className="settings-section">
          <div className="settings-heading">
            <div>
              <Typography.Title level={5}>应用更新</Typography.Title>
              <Typography.Text type="secondary">手动下载并安装经过签名验证的更新</Typography.Text>
            </div>
            <Tag>{updateState.currentVersion ? `v${updateState.currentVersion}` : "读取版本中"}</Tag>
          </div>

          <div
            className={`update-status update-status-${updateState.phase}`}
            role={updateState.phase === "error" ? "alert" : "status"}
            aria-live={updateState.phase === "error" ? "assertive" : "polite"}
            aria-busy={updateBusy}
          >
            {updateState.phase === "idle" && (
              <Typography.Paragraph type="secondary">
                Tick 只会在你点击检查后访问更新服务，不会后台下载或静默安装。
              </Typography.Paragraph>
            )}

            {updateState.phase === "checking" && (
              <Typography.Paragraph>正在检查更新…</Typography.Paragraph>
            )}

            {updateState.phase === "current" && (
              <Alert type="success" showIcon title={`当前 v${updateState.currentVersion ?? "—"} 已是最新版`} />
            )}

            {updateState.update && ["available", "downloading", "installing", "ready", "windows-installer"].includes(updateState.phase) && (
              <div className="update-release">
                <div className="update-release-heading">
                  <strong>发现 v{updateState.update.version}</strong>
                  {updateState.update.date ? <span>{formatReleaseDate(updateState.update.date)}</span> : null}
                </div>
                <Typography.Paragraph className="update-release-notes">
                  {updateState.update.body?.trim() || "此版本没有附加说明。"}
                </Typography.Paragraph>
              </div>
            )}

            {updateState.phase === "downloading" && (
              <div className="update-progress" aria-label="更新下载进度">
                {percent === undefined ? (
                  <>
                    <div className="update-progress-indeterminate" aria-hidden="true"><i /></div>
                    <span>正在下载，服务器未提供总大小 · 已接收 {formatBytes(updateState.progress.downloaded)}</span>
                  </>
                ) : (
                  <>
                    <Progress percent={percent} showInfo={false} strokeColor="#20201f" trailColor="#e1e1de" />
                    <span>{percent}% · {formatBytes(updateState.progress.downloaded)} / {formatBytes(updateState.progress.total ?? 0)}</span>
                  </>
                )}
              </div>
            )}

            {updateState.phase === "installing" && (
              <Alert
                type="info"
                showIcon
                title={windows ? "正在验证签名并交给 Windows 安装器" : "正在验证签名并安装"}
                description={windows ? "验证通过后 Tick 会按安装器限制关闭，请在安装窗口中完成更新。" : "验证失败会立即停止，不会安装未验证的软件。"}
              />
            )}

            {updateState.phase === "windows-installer" && (
              <Alert type="info" showIcon title="Windows 安装器已启动" description="请按照安装器窗口完成更新；重启时机由安装器控制。" />
            )}

            {updateState.phase === "ready" && (
              <Alert type="success" showIcon title="更新已安装" description="新版本会在你明确重启 Tick 后生效。" />
            )}

            {updateState.phase === "error" && (
              <Alert type="error" showIcon title="更新没有完成" description={updateState.error} />
            )}
          </div>

          <Space wrap>
            {["idle", "checking", "current"].includes(updateState.phase) && (
              <Button type="primary" icon={<ReloadOutlined />} loading={updateState.phase === "checking"} disabled={updateBusy} onClick={checkUpdate}>
                {updateState.phase === "current" ? "再次检查" : "检查更新"}
              </Button>
            )}
            {updateState.phase === "available" && (
              <Button type="primary" icon={<CloudDownloadOutlined />} onClick={downloadAndInstallUpdate}>
                下载并安装 v{updateState.update?.version}
              </Button>
            )}
            {updateState.phase === "ready" && (
              <Button type="primary" icon={<ReloadOutlined />} onClick={restartAfterUpdate}>
                重启并完成更新
              </Button>
            )}
            {updateState.phase === "error" && (
              <>
                <Button type="primary" icon={<ReloadOutlined />} onClick={checkUpdate}>重新检查</Button>
                <Button icon={<LinkOutlined />} onClick={openReleasesRecovery}>打开 Releases 手动恢复</Button>
              </>
            )}
          </Space>
        </div>

        <div className="settings-section">
          <div className="settings-heading">
            <div>
              <Typography.Title level={5}>DeepSeek</Typography.Title>
              <Typography.Text type="secondary">用于根据描述生成完整自动化任务</Typography.Text>
            </div>
            {status.configured ? (
              <Tag color="success" icon={<CheckCircleFilled />}>
                已配置 {status.maskedHint}
              </Tag>
            ) : (
              <Tag>未配置</Tag>
            )}
          </div>

          <Typography.Paragraph type="secondary" className="settings-storage-note">
            保存后会自动读取。Key 存在当前用户的应用设置目录，不会写入项目源码或日志。
          </Typography.Paragraph>

          <div className="settings-key-row">
            <Input.Password
              value={apiKey}
              prefix={<KeyOutlined />}
              placeholder={status.configured ? "输入新 Key 可替换当前配置" : "粘贴 DeepSeek API Key"}
              autoComplete="new-password"
              onChange={(event) => setApiKey(event.target.value)}
              onPressEnter={saveKey}
            />
            <Button type="primary" loading={loading} onClick={saveKey}>
              {status.configured ? "替换" : "保存"}
            </Button>
          </div>

          {status.configured && (
            <Space>
              <Button loading={testing} onClick={testConnection}>
                测试连接
              </Button>
              <Popconfirm
                title="删除 DeepSeek API Key？"
                description="删除后，AI 脚本生成功能将不可用。"
                okText="删除"
                cancelText="取消"
                onConfirm={deleteKey}
              >
                <Button danger icon={<DeleteOutlined />}>
                  删除配置
                </Button>
              </Popconfirm>
            </Space>
          )}
        </div>
      </div>
    </Modal>
  );
}

function formatBytes(bytes: number) {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function formatReleaseDate(value: string) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return new Intl.DateTimeFormat("zh-CN", { year: "numeric", month: "short", day: "numeric" }).format(date);
}
