import {
  CheckCircleFilled,
  DeleteOutlined,
  KeyOutlined,
  InfoCircleOutlined,
} from "@ant-design/icons";
import { Alert, Button, Input, Modal, Popconfirm, Space, Tag, Typography, message } from "antd";
import { useCallback, useEffect, useState } from "react";
import {
  deleteDeepSeekApiKey,
  getDeepSeekConfigStatus,
  saveDeepSeekApiKey,
  testDeepSeekConnection,
  type DeepSeekConfigStatus,
} from "../services/deepseek";
import { friendlyError } from "../utils/errors";

interface SettingsModalProps {
  open: boolean;
  onClose: () => void;
}

export function SettingsModal({ open, onClose }: SettingsModalProps) {
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

        <Alert
          type="info"
          showIcon
          icon={<InfoCircleOutlined />}
          title="保存在这台电脑上"
          description="保存一次后 Tick 会自动读取。Key 存在仅当前用户可读写的应用设置文件中，不会进入项目源码或日志。"
        />

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
    </Modal>
  );
}
