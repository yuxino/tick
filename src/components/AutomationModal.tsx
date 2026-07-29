import { RobotOutlined, SafetyCertificateOutlined } from "@ant-design/icons";
import { Alert, Button, Input, Modal, Space, Typography, message } from "antd";
import { useState } from "react";
import { generateAutomation, type AutomationDraft } from "../services/launchd";
import { friendlyError } from "../utils/errors";

interface AutomationModalProps {
  open: boolean;
  onCancel: () => void;
  onGenerated: (draft: AutomationDraft) => void;
}

const EXAMPLES = [
  "每天 9 点提醒我喝水，并把提醒时间写进日志",
  "每天晚上整理桌面截图，移动到桌面的 Screenshots 文件夹",
  "每隔 30 分钟检查一个网站是否能访问，失败时发系统通知",
];

export function AutomationModal({ open, onCancel, onGenerated }: AutomationModalProps) {
  const [prompt, setPrompt] = useState("");
  const [generating, setGenerating] = useState(false);

  async function generate() {
    if (!prompt.trim()) {
      message.warning("先说说你想让 Mac 自动做什么");
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
      title="让 Tick 帮你搭一个自动化"
      width={640}
      footer={
        <Space>
          <Button onClick={onCancel}>取消</Button>
          <Button type="primary" icon={<RobotOutlined />} loading={generating} onClick={generate}>
            生成自动化
          </Button>
        </Space>
      }
      onCancel={onCancel}
      destroyOnHidden
    >
      <div className="automation-composer">
        <div>
          <Typography.Title level={3}>想让 Mac 定时做什么？</Typography.Title>
          <Typography.Paragraph type="secondary">
            时间、脚本和任务说明都可以一起说。Tick 会先生成草稿，不会直接启用。
          </Typography.Paragraph>
        </div>
        <Input.TextArea
          value={prompt}
          autoFocus
          autoSize={{ minRows: 5, maxRows: 10 }}
          placeholder="例如：每天晚上 11 点，把下载目录里超过 30 天的安装包移到废纸篓，完成后发一条通知。"
          onChange={(event) => setPrompt(event.target.value)}
        />
        <div className="automation-examples">
          {EXAMPLES.map((example) => (
            <button key={example} type="button" onClick={() => setPrompt(example)}>
              {example}
            </button>
          ))}
        </div>
        <Alert
          type="info"
          showIcon
          icon={<SafetyCertificateOutlined />}
          title="生成后先检查，再试运行"
          description="涉及文件移动、联网或私人目录时，Tick 会把风险单独列出来。"
        />
      </div>
    </Modal>
  );
}
