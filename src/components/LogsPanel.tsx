import { ClearOutlined, ReloadOutlined } from "@ant-design/icons";
import CodeMirror from "@uiw/react-codemirror";
import { Alert, Button, Popconfirm, Space, Switch, Tabs, Typography } from "antd";
import { useCallback, useEffect, useState } from "react";
import { clearLaunchdLog, readLaunchdLog } from "../services/launchd";
import type { JobLog, LaunchdJob, LogKind } from "../types/launchd";

interface LogsPanelProps {
  job?: LaunchdJob;
}

export function LogsPanel({ job }: LogsPanelProps) {
  const [kind, setKind] = useState<LogKind>("stdout");
  const [log, setLog] = useState<JobLog>();
  const [loading, setLoading] = useState(false);
  const [autoRefresh, setAutoRefresh] = useState(false);
  const [error, setError] = useState<string>();

  const loadLog = useCallback(async () => {
    if (!job) return;
    setLoading(true);
    setError(undefined);
    try {
      setLog(await readLaunchdLog(job.id, kind));
    } catch (err) {
      setError(String(err));
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
    await clearLaunchdLog(job.id, kind);
    await loadLog();
  }

  if (!job) {
    return <div className="empty-detail">Select a job to inspect logs.</div>;
  }

  return (
    <Space direction="vertical" size={12} className="full-width">
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
          <Typography.Text type="secondary">Auto</Typography.Text>
          <Switch size="small" checked={autoRefresh} onChange={setAutoRefresh} />
          <Button icon={<ReloadOutlined />} onClick={loadLog} loading={loading} />
          <Popconfirm title="Clear this log?" okText="Clear" onConfirm={handleClear}>
            <Button icon={<ClearOutlined />} />
          </Popconfirm>
        </Space>
      </div>

      {error && <Alert type="error" message={error} showIcon />}
      {log?.truncated && <Alert type="warning" message="Showing the tail of a large log file." showIcon />}

      <Typography.Text type="secondary" className="path-line">
        {log?.path ?? (kind === "stdout" ? job.stdoutPath : job.stderrPath)}
      </Typography.Text>
      <CodeMirror
        value={log?.content ?? ""}
        height="420px"
        editable={false}
        basicSetup={{ lineNumbers: true, foldGutter: false }}
      />
    </Space>
  );
}
