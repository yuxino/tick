import { ReloadOutlined } from "@ant-design/icons";
import { xml } from "@codemirror/lang-xml";
import CodeMirror from "@uiw/react-codemirror";
import { Alert, Button, Space, Tooltip, Typography } from "antd";
import { useCallback } from "react";
import { tickEditorTheme } from "../editorTheme";
import { readJobDefinition } from "../services/scheduler";
import type { ScheduledJob, SchedulerCapabilities } from "../types/scheduler";
import { friendlyError } from "../utils/errors";
import { useAsyncResource } from "../hooks/useAsyncResource";
import { displayPath } from "../utils/paths";

interface DefinitionPanelProps {
  job?: ScheduledJob;
  capabilities: SchedulerCapabilities;
}

export function DefinitionPanel({ job, capabilities }: DefinitionPanelProps) {
  const jobId = job?.id;
  const read = useCallback(() => jobId ? readJobDefinition(jobId) : Promise.resolve(""), [jobId]);
  const { value: content, loading, error, refresh: loadDefinition } = useAsyncResource(`${jobId ?? ""}:${job?.lastModifiedAt ?? ""}`, read);

  if (!job) {
    return <div className="empty-detail">选择一个任务查看 {capabilities.definitionLabel}。</div>;
  }

  return (
    <Space orientation="vertical" size={12} className="full-width">
      <div className="panel-toolbar compact">
        <Typography.Text type="secondary" className="path-line">
          {displayPath(job.definitionPath, capabilities.homeDirectory)}
        </Typography.Text>
        <Tooltip title={`刷新 ${capabilities.definitionLabel}`}>
          <Button icon={<ReloadOutlined />} onClick={loadDefinition} loading={loading} />
        </Tooltip>
      </div>
      {!!error && <Alert type="error" title={friendlyError(error)} showIcon />}
      <CodeMirror value={content ?? ""} height="480px" extensions={[tickEditorTheme, xml()]} editable={false} basicSetup={{ lineNumbers: true }} />
    </Space>
  );
}
