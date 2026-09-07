import { ClearOutlined, ReloadOutlined } from "@ant-design/icons";
import CodeMirror from "@uiw/react-codemirror";
import { Alert, Button, Popconfirm, Space, Switch, Tabs, Tooltip, Typography } from "antd";
import { useCallback, useState } from "react";
import { clearScheduledJobLog, readScheduledJobLog } from "../services/scheduler";
import { tickEditorTheme } from "../editorTheme";
import type { LogKind, ScheduledJob } from "../types/scheduler";
import { friendlyError } from "../utils/errors";
import { useAsyncResource } from "../hooks/useAsyncResource";
import { displayPath } from "../utils/paths";

interface LogsPanelProps {
  job?: ScheduledJob;
  homeDirectory: string;
  active: boolean;
}

export function LogsPanel({ job, homeDirectory, active }: LogsPanelProps) {
  const [kind, setKind] = useState<LogKind>("stdout");
  const [clearing, setClearing] = useState(false);
  const [autoRefresh, setAutoRefresh] = useState(true);
  const [clearError, setClearError] = useState<{ key: string; message: string }>();
  const jobId = job?.id;
  const resourceKey = `${jobId ?? ""}:${kind}`;
  const read = useCallback(() => jobId ? readScheduledJobLog(jobId, kind) : Promise.resolve(undefined), [jobId, kind]);
  const { value: log, loading, error: readError, refresh: loadLog } = useAsyncResource(resourceKey, read, autoRefresh && jobId ? 2000 : undefined, active);
  const error = clearError?.key === resourceKey ? clearError.message : readError ? friendlyError(readError) : undefined;

  async function handleClear() {
    if (!job) return;
    setClearing(true);
    setClearError(undefined);
    try {
      await clearScheduledJobLog(job.id, kind);
      await loadLog();
    } catch (err) {
      setClearError({ key: resourceKey, message: friendlyError(err) });
    } finally {
      setClearing(false);
    }
  }

  if (!job) {
    return <div className="empty-detail">选择一个任务查看日志。</div>;
  }

  return (
    <Space orientation="vertical" size={12} className="full-width">
      <div className="panel-toolbar compact">
        <Tabs
          activeKey={kind}
          onChange={(key) => setKind(key as LogKind)}
          items={[
            { key: "stdout", label: "stdout" },
            { key: "stderr", label: "stderr" },
          ]}
        />
        <Space>
          <Typography.Text type="secondary">自动刷新</Typography.Text>
          <Switch size="small" checked={autoRefresh} onChange={setAutoRefresh} />
          <Tooltip title="刷新日志">
            <Button icon={<ReloadOutlined />} onClick={loadLog} loading={loading} />
          </Tooltip>
          <Popconfirm title="清空这份日志？" okText="清空" cancelText="取消" onConfirm={handleClear}>
            <Tooltip title="清空日志">
              <Button icon={<ClearOutlined />} loading={clearing} />
            </Tooltip>
          </Popconfirm>
        </Space>
      </div>

      {error && <Alert type="error" title={error} showIcon />}
      {log?.truncated && <Alert type="warning" title="日志文件太大，当前只显示末尾内容。" showIcon />}

      <Typography.Text type="secondary" className="path-line">
        {displayPath(log?.path ?? (kind === "stdout" ? job.stdoutPath : job.stderrPath), homeDirectory)}
      </Typography.Text>
      <CodeMirror
        value={log?.content ?? ""}
        height="420px"
        extensions={[tickEditorTheme]}
        editable={false}
        basicSetup={{ lineNumbers: true, foldGutter: false }}
      />
    </Space>
  );
}
