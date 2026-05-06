import { ReloadOutlined } from "@ant-design/icons";
import { xml } from "@codemirror/lang-xml";
import CodeMirror from "@uiw/react-codemirror";
import { Alert, Button, Space, Tooltip, Typography } from "antd";
import { useCallback, useEffect, useState } from "react";
import { readLaunchdPlist } from "../services/launchd";
import type { LaunchdJob } from "../types/launchd";
import { friendlyError } from "../utils/errors";

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
      setError(friendlyError(err));
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
    return <div className="empty-detail">选择一个任务查看 plist 配置。</div>;
  }

  return (
    <Space orientation="vertical" size={12} className="full-width">
      <div className="panel-toolbar compact">
        <Typography.Text type="secondary" className="path-line">
          {job.plistPath}
        </Typography.Text>
        <Tooltip title="刷新 plist">
          <Button icon={<ReloadOutlined />} onClick={loadPlist} loading={loading} />
        </Tooltip>
      </div>
      {error && <Alert type="error" title={error} showIcon />}
      <CodeMirror value={content} height="480px" extensions={[xml()]} editable={false} basicSetup={{ lineNumbers: true }} />
    </Space>
  );
}
