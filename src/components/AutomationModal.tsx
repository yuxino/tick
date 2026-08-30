import { Button, Input, Modal, Space, Typography, message } from "antd";
import { useState } from "react";
import { generateAutomation, type AutomationDraft } from "../services/scheduler";
import type { SchedulerCapabilities } from "../types/scheduler";
import { friendlyError } from "../utils/errors";

interface AutomationModalProps {
  open: boolean;
  capabilities: SchedulerCapabilities;
  onCancel: () => void;
  onManual: () => void;
  onGenerated: (draft: AutomationDraft) => void;
}

export function AutomationModal({ open, capabilities, onCancel, onManual, onGenerated }: AutomationModalProps) {
  const [prompt, setPrompt] = useState("");
  const [generating, setGenerating] = useState(false);

  async function generate() {
    if (!prompt.trim()) {
      message.warning(`先说说你想让 ${capabilities.computerLabel} 自动做什么`);
      return;
    }
    setGenerating(true);
    try {
      const draft = await generateAutomation(prompt);
      onGenerated(draft);
      setPrompt("");
      message.success("自动化草稿已经搭好了");
    } catch (error) {
      message.error(friendlyError(error));
    } finally {
      setGenerating(false);
    }
  }

  return (
    <Modal
      open={open}
      title="新建任务"
      width={560}
      footer={
        <Space>
          <Button onClick={onManual}>手动填写</Button>
          <Button type="primary" loading={generating} onClick={generate}>
            继续
          </Button>
        </Space>
      }
      onCancel={onCancel}
      destroyOnHidden
    >
      <div className="automation-composer">
        <div>
          <Typography.Title level={4}>描述要自动完成的事情</Typography.Title>
          <Typography.Paragraph type="secondary">可以写上运行时间。下一步会先生成草稿供你检查。</Typography.Paragraph>
        </div>
        <Input.TextArea
          value={prompt}
          autoFocus
          autoSize={{ minRows: 4, maxRows: 8 }}
          placeholder={`例如：每天晚上 11 点，把下载目录里超过 30 天的安装包移到${capabilities.trashLabel}，完成后发一条通知。`}
          onChange={(event) => setPrompt(event.target.value)}
        />
      </div>
    </Modal>
  );
}
