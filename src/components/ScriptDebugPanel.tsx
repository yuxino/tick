import { CheckCircleOutlined, CloseCircleOutlined, ClockCircleOutlined } from "@ant-design/icons";
import { Typography } from "antd";
import type { RunNodeScriptDebugResponse } from "../services/launchd";

interface ScriptDebugPanelProps {
  result?: RunNodeScriptDebugResponse;
}

export function ScriptDebugPanel({ result }: ScriptDebugPanelProps) {
  if (!result) {
    return (
      <div className="debug-panel empty">
        <Typography.Text type="secondary">点“调试运行”后，这里会显示 stdout、stderr 和退出码。</Typography.Text>
      </div>
    );
  }

  const ok = !result.timedOut && result.exitCode === 0;

  return (
    <div className={`debug-panel ${ok ? "ok" : "failed"}`}>
      <div className="debug-summary">
        <span className="debug-status">
          {ok ? <CheckCircleOutlined /> : <CloseCircleOutlined />}
          {result.timedOut ? "超时" : `退出码 ${result.exitCode ?? "未知"}`}
        </span>
        <span className="debug-time">
          <ClockCircleOutlined />
          {result.durationMs}ms
        </span>
      </div>

      <DebugBlock title="stdout" content={result.stdout} emptyText="没有标准输出" />
      <DebugBlock title="stderr" content={result.stderr} emptyText="没有错误输出" />
    </div>
  );
}

function DebugBlock({ title, content, emptyText }: { title: string; content: string; emptyText: string }) {
  return (
    <div className="debug-block">
      <span>{title}</span>
      <pre>{content.trim() || emptyText}</pre>
    </div>
  );
}
