import { ReloadOutlined } from "@ant-design/icons";
import { xml } from "@codemirror/lang-xml";
import CodeMirror from "@uiw/react-codemirror";
import { Alert, Button, Space, Typography } from "antd";
import { useCallback, useEffect, useState } from "react";
import { readLaunchdPlist } from "../services/launchd";
import type { LaunchdJob } from "../types/launchd";

interface PlistPanelProps {
  job?: LaunchdJob;
}

export function PlistPanel({ job }: PlistPanelProps) {
  const [content, setContent] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string>();

  const loadPlist = useCallback(async () => {
    if (!job) return;
    setLoading(true);
    setError(undefined);
    try {
      setContent(await readLaunchdPlist(job.id));
    } catch (err) {
      setError(String(err));
      setContent("");
    } finally {
      setLoading(false);
    }
  }, [job]);

  useEffect(() => {
    setContent("");
    loadPlist();
  }, [loadPlist]);

  if (!job) {
    return <div className="empty-detail">Select a job to inspect its plist.</div>;
  }

  return (
    <Space direction="vertical" size={12} className="full-width">
      <div className="panel-toolbar compact">
        <Typography.Text type="secondary" className="path-line">
          {job.plistPath}
        </Typography.Text>
        <Button icon={<ReloadOutlined />} onClick={loadPlist} loading={loading} />
      </div>
      {error && <Alert type="error" message={error} showIcon />}
      <CodeMirror value={content} height="480px" extensions={[xml()]} editable={false} basicSetup={{ lineNumbers: true }} />
    </Space>
  );
}
