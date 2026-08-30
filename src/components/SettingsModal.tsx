import {
  CheckCircleFilled,
  CodeOutlined,
  DeleteOutlined,
  KeyOutlined,
  ReloadOutlined,
  WarningFilled,
} from "@ant-design/icons";
import { Button, Input, Modal, Popconfirm, Space, Tag, Typography, message } from "antd";
import { useCallback, useEffect, useState } from "react";
import {
  deleteDeepSeekApiKey,
  getDeepSeekConfigStatus,
  saveDeepSeekApiKey,
  testDeepSeekConnection,
  type DeepSeekConfigStatus,
} from "../services/deepseek";
import { friendlyError } from "../utils/errors";
import type { NodeRuntimeStatus } from "../types/scheduler";

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
    }
  }, [loadStatus, open]);

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
      width={520}
      footer={<Button onClick={onClose}>完成</Button>}
      onCancel={onClose}
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
