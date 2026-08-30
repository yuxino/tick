import { ClearOutlined, ReloadOutlined } from "@ant-design/icons";
import CodeMirror from "@uiw/react-codemirror";
import { Alert, Button, Popconfirm, Space, Switch, Tabs, Tooltip, Typography } from "antd";
import { useCallback, useEffect, useState } from "react";
import { clearScheduledJobLog, readScheduledJobLog } from "../services/scheduler";
import { tickEditorTheme } from "../editorTheme";
import type { JobLog, LogKind, ScheduledJob } from "../types/scheduler";
import { friendlyError } from "../utils/errors";
import { displayPath } from "../utils/paths";

interface LogsPanelProps {
  job?: ScheduledJob;
  homeDirectory: string;
}

export function LogsPanel({ job, homeDirectory }: LogsPanelProps) {
  const [kind, setKind] = useState<LogKind>("stdout");
  const [log, setLog] = useState<JobLog>();
  const [loading, setLoading] = useState(false);
  const [clearing, setClearing] = useState(false);
  const [autoRefresh, setAutoRefresh] = useState(true);
  const [error, setError] = useState<string>();

  const loadLog = useCallback(async () => {
    if (!job) return;
    setLoading(true);
    setError(undefined);
    try {
      setLog(await readScheduledJobLog(job.id, kind));
    } catch (err) {
      setError(friendlyError(err));
    } finally {
      setLoading(false);
    }
  }, [job, kind]);

  useEffect(() => {
    setLog(undefined);
    loadLog();
  }, [loadLog]);

  useEffect(() => {
    if (!autoRefresh || !job) return;
    const timer = window.setInterval(loadLog, 2000);
    return () => window.clearInterval(timer);
  }, [autoRefresh, job, loadLog]);

  async function handleClear() {
    if (!job) return;
    setClearing(true);
    setError(undefined);
    try {
      await clearScheduledJobLog(job.id, kind);
      await loadLog();
    } catch (err) {
      setError(friendlyError(err));
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
