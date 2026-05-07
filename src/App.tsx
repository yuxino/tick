import { DeleteOutlined, EditOutlined, FileTextOutlined, PlayCircleOutlined, PlusOutlined, RobotOutlined } from "@ant-design/icons";
import { javascript } from "@codemirror/lang-javascript";
import CodeMirror from "@uiw/react-codemirror";
import { Alert, Button, Descriptions, Input, Layout, message, Popconfirm, Space, Switch, Tabs, Tag, Typography } from "antd";
import { useCallback, useEffect, useMemo, useState } from "react";
import { JobFormModal } from "./components/JobFormModal";
import { JobsTable } from "./components/JobsTable";
import { LogsPanel } from "./components/LogsPanel";
import { MikuMascot } from "./components/MikuMascot";
import { PlistPanel } from "./components/PlistPanel";
import { ScriptDebugPanel } from "./components/ScriptDebugPanel";
import { tickEditorTheme } from "./editorTheme";
import {
  deleteLaunchdJob,
  disableLaunchdJob,
  enableLaunchdJob,
  generateNodeScript,
  listLaunchdJobs,
  runNodeScriptDebug,
  runLaunchdJobNow,
  saveLaunchdJob,
} from "./services/launchd";
import type { RunNodeScriptDebugResponse } from "./services/launchd";
import type { LaunchdJob, LaunchdJobInput } from "./types/launchd";
import { friendlyError } from "./utils/errors";
import { commandSummary, emptyJobInput, scheduleSummary, statusLabel, toJobInput } from "./utils/launchd";

const { Header, Content } = Layout;

function App() {
  const [jobs, setJobs] = useState<LaunchdJob[]>([]);
  const [selectedId, setSelectedId] = useState<string>();
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [busyId, setBusyId] = useState<string>();
  const [error, setError] = useState<string>();
  const [formOpen, setFormOpen] = useState(false);
  const [editingJob, setEditingJob] = useState<LaunchdJob>();

  const selectedJob = useMemo(
    () => jobs.find((job) => job.id === selectedId) ?? jobs[0],
    [jobs, selectedId],
  );
  const stats = useMemo(
    () => ({
      total: jobs.length,
      enabled: jobs.filter((job) => job.status === "enabled").length,
      disabled: jobs.filter((job) => job.status === "disabled").length,
      issues: jobs.filter((job) => job.status === "missing" || job.status === "error").length,
    }),
    [jobs],
  );

  const loadJobs = useCallback(async () => {
    setLoading(true);
    setError(undefined);
    try {
      const nextJobs = await listLaunchdJobs();
      setJobs(nextJobs);
      setSelectedId((current) => {
        if (current && nextJobs.some((job) => job.id === current)) return current;
        return nextJobs[0]?.id;
      });
    } catch (err) {
      setError(friendlyError(err));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadJobs();
  }, [loadJobs]);

  function openCreate() {
    setEditingJob(undefined);
    setFormOpen(true);
  }

  function openEdit(job: LaunchdJob) {
    setEditingJob(job);
    setFormOpen(true);
  }

  async function handleSave(input: LaunchdJobInput) {
    setSaving(true);
    try {
      const saved = await saveLaunchdJob(input);
      message.success("任务已保存");
      setFormOpen(false);
      setEditingJob(undefined);
      await loadJobs();
      setSelectedId(saved.id);
    } catch (err) {
      message.error(friendlyError(err));
    } finally {
      setSaving(false);
    }
  }

  async function handleQuickScriptSave(input: LaunchdJobInput) {
    await handleSave(input);
  }

  async function withBusy(job: LaunchdJob, action: () => Promise<void>, success: string) {
    setBusyId(job.id);
    try {
      await action();
      message.success(success);
      await loadJobs();
      setSelectedId(job.id);
    } catch (err) {
      message.error(friendlyError(err));
    } finally {
      setBusyId(undefined);
    }
  }

  function handleToggle(job: LaunchdJob, enabled: boolean) {
    withBusy(
      job,
      async () => {
        if (enabled) {
          await enableLaunchdJob(job.id);
        } else {
          await disableLaunchdJob(job.id);
        }
      },
      enabled ? "任务已启用" : "任务已停用",
    );
  }

  function handleRun(job: LaunchdJob) {
    withBusy(job, () => runLaunchdJobNow(job.id), "任务已启动");
  }

  async function handleDelete(job: LaunchdJob) {
    setBusyId(job.id);
    try {
      await deleteLaunchdJob(job.id);
      message.success("任务已删除");
      await loadJobs();
      setSelectedId(undefined);
    } catch (err) {
      message.error(friendlyError(err));
    } finally {
      setBusyId(undefined);
    }
  }

  return (
    <Layout className="app-shell">
      <Header className="app-header">
        <div className="window-drag-region" data-tauri-drag-region />
        <div className="brand-block">
          <div className="app-title">Tick</div>
        </div>
        <div className="header-controls">
          <div className="header-stats" aria-label="任务状态概览">
            <StatusPill label="全部" value={stats.total} />
            <StatusPill label="运行中" value={stats.enabled} tone="good" />
            <StatusPill label="停用" value={stats.disabled} />
            <StatusPill label="异常" value={stats.issues} tone={stats.issues > 0 ? "warn" : "quiet"} />
          </div>
          <Space className="header-actions">
            <Button icon={<PlusOutlined />} type="primary" onClick={openCreate}>
              新建任务
            </Button>
          </Space>
        </div>
      </Header>

      <Content className="app-content">
        {error && <Alert type="error" title={error} showIcon className="top-alert" />}

        {jobs.length === 0 && !loading ? (
          <QuickScriptCreator saving={saving} onSubmit={handleQuickScriptSave} onAdvanced={openCreate} />
        ) : (
          <div className="workspace">
            <JobsTable
              jobs={jobs}
              selectedId={selectedJob?.id}
              loading={loading}
              busyId={busyId}
              onSelect={(job) => setSelectedId(job.id)}
              onEdit={openEdit}
              onRefresh={loadJobs}
              onToggle={handleToggle}
              onRun={handleRun}
              onDelete={handleDelete}
            />

            <div className="panel detail-panel">
              {selectedJob ? (
                <DetailPanel
                  job={selectedJob}
                  busy={busyId === selectedJob.id}
                  onToggle={handleToggle}
                  onRun={handleRun}
                  onEdit={openEdit}
                  onDelete={handleDelete}
                />
              ) : (
                <div className="empty-detail">选择一个任务查看详情。</div>
              )}
            </div>
          </div>
        )}
      </Content>

      <JobFormModal
        open={formOpen}
        initialValue={editingJob ? toJobInput(editingJob) : emptyJobInput()}
        saving={saving}
        onCancel={() => {
          setFormOpen(false);
          setEditingJob(undefined);
        }}
        onSubmit={handleSave}
      />
      <MikuMascot />
    </Layout>
  );
}

function QuickScriptCreator({
  saving,
  onSubmit,
  onAdvanced,
}: {
  saving: boolean;
  onSubmit: (input: LaunchdJobInput) => Promise<void>;
  onAdvanced: () => void;
}) {
  const [name, setName] = useState("我的脚本任务");
  const [time, setTime] = useState("09:00:00");
  const [script, setScript] = useState(DEFAULT_NODE_SCRIPT);
  const [aiPrompt, setAiPrompt] = useState("");
  const [generating, setGenerating] = useState(false);
  const [debugging, setDebugging] = useState(false);
  const [debugResult, setDebugResult] = useState<RunNodeScriptDebugResponse>();

  async function handleGenerateScript() {
    if (!aiPrompt.trim()) {
      message.warning("先写一句你想让脚本做什么");
      return;
    }
    setGenerating(true);
    try {
      const result = await generateNodeScript({ prompt: aiPrompt, currentScript: script });
      setScript(result.script);
      message.success("脚本写好了");
    } catch (err) {
      message.error(friendlyError(err));
    } finally {
      setGenerating(false);
    }
  }

  async function handleDebugScript() {
    if (!script.trim()) {
      message.warning("没有可运行的脚本内容");
      return;
    }
    setDebugging(true);
    try {
      const result = await runNodeScriptDebug({ script });
      setDebugResult(result);
      if (result.exitCode === 0 && !result.timedOut) {
        message.success("调试运行完成");
      } else {
        message.warning("调试运行结束，检查输出");
      }
    } catch (err) {
      message.error(friendlyError(err));
    } finally {
      setDebugging(false);
    }
  }

  async function submit() {
    const [hour = "0", minute = "0", second = "0"] = time.split(":");
    const input = emptyJobInput();
    input.name = name.trim() || "我的脚本任务";
    input.description = "直接在 Tick 里创建的 Node.js 脚本";
    input.schedule.calendar.hour = Number(hour);
    input.schedule.calendar.minute = Number(minute);
    input.schedule.calendar.second = Number(second);
    input.execution.mode = "inline_shell";
    input.execution.interpreter = "/usr/bin/env node";
    input.execution.inlineScript = script.trim() || DEFAULT_NODE_SCRIPT;
    await onSubmit(input);
  }

  return (
    <div className="quick-create-shell">
      <div className="quick-create-copy">
        <Typography.Text type="secondary">Neon Node Atelier</Typography.Text>
        <Typography.Title level={2}>直接写 Node.js，Tick 来定时跑</Typography.Title>
        <Typography.Paragraph type="secondary">
          默认每天运行一次。AI 帮你起草脚本，调试面板先听一遍输出，再保存成 macOS 定时任务。
        </Typography.Paragraph>
      </div>

      <div className="quick-create-panel panel">
        <div className="quick-create-fields">
          <Input value={name} onChange={(event) => setName(event.target.value)} placeholder="任务名称" />
          <Input type="time" step={1} value={time} onChange={(event) => setTime(event.target.value)} />
        </div>
        <CodeMirror
          value={script}
          height="300px"
          extensions={[tickEditorTheme, javascript({ jsx: true, typescript: true })]}
          basicSetup={{ lineNumbers: true, foldGutter: true }}
          onChange={setScript}
        />
        <div className="ai-helper">
          <Input.TextArea
            value={aiPrompt}
            autoSize={{ minRows: 2, maxRows: 4 }}
            placeholder="告诉 AI 你想让这个定时任务做什么，比如：每天 9 点提醒我喝水，并在日志里写时间。"
            onChange={(event) => setAiPrompt(event.target.value)}
          />
          <Button icon={<RobotOutlined />} loading={generating} onClick={handleGenerateScript}>
            AI 帮我写
          </Button>
        </div>
        <div className="debug-toolbar">
          <Button icon={<PlayCircleOutlined />} loading={debugging} onClick={handleDebugScript}>
            调试运行
          </Button>
          <Typography.Text type="secondary">先跑一次，看结果，再保存成定时任务。</Typography.Text>
        </div>
        <ScriptDebugPanel result={debugResult} />
        <div className="quick-create-actions">
          <Typography.Text type="secondary">生成后可以继续手改，保存时会按上面的时间定时运行。</Typography.Text>
          <Space>
            <Button onClick={onAdvanced}>高级编辑</Button>
            <Button type="primary" icon={<PlusOutlined />} loading={saving} onClick={submit}>
              保存任务
            </Button>
          </Space>
        </div>
      </div>
    </div>
  );
}

const DEFAULT_NODE_SCRIPT = `console.log("tick", new Date().toLocaleString());`;

function StatusPill({ label, value, tone = "quiet" }: { label: string; value: number; tone?: "quiet" | "good" | "warn" }) {
  return (
    <div className={`status-pill ${tone}`}>
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}

function DetailPanel({
  job,
  busy,
  onToggle,
  onRun,
  onEdit,
  onDelete,
}: {
  job: LaunchdJob;
  busy: boolean;
  onToggle: (job: LaunchdJob, enabled: boolean) => void;
  onRun: (job: LaunchdJob) => void;
  onEdit: (job: LaunchdJob) => void;
  onDelete: (job: LaunchdJob) => void;
}) {
  const [activeTab, setActiveTab] = useState("overview");

  useEffect(() => {
    setActiveTab("overview");
  }, [job.id]);

  return (
    <>
      <div className="detail-hero">
        <div className="detail-heading">
          <Space size={8} wrap>
            <Tag color={job.status === "enabled" ? "green" : job.status === "missing" ? "orange" : "default"}>
              {statusLabel(job.status)}
            </Tag>
            <Typography.Text type="secondary" className="mono-subtle">
              {job.label}
            </Typography.Text>
          </Space>
          <Typography.Title level={3}>{job.name}</Typography.Title>
          <Typography.Text type="secondary">{job.description || "没有描述"}</Typography.Text>
        </div>
        <Space wrap className="detail-actions">
          <Switch
            checked={job.status === "enabled"}
            checkedChildren="启用"
            unCheckedChildren="停用"
            loading={busy}
            onChange={(checked) => onToggle(job, checked)}
          />
          <Button icon={<PlayCircleOutlined />} loading={busy} onClick={() => onRun(job)}>
            立即运行
          </Button>
          <Button icon={<FileTextOutlined />} onClick={() => setActiveTab("logs")}>
            查看日志
          </Button>
          <Button icon={<EditOutlined />} onClick={() => onEdit(job)}>
            编辑
          </Button>
          <Popconfirm title="删除这个任务？" okText="删除" cancelText="取消" okButtonProps={{ danger: true }} onConfirm={() => onDelete(job)}>
            <Button danger icon={<DeleteOutlined />} loading={busy}>
              删除
            </Button>
          </Popconfirm>
        </Space>
      </div>

      <div className="detail-quick-grid">
        <InfoTile label="运行计划" value={scheduleSummary(job.schedule)} />
        <InfoTile label="执行命令" value={commandSummary(job.execution)} />
        <InfoTile label="最近更新" value={new Date(job.lastModifiedAt).toLocaleString("zh-CN")} />
      </div>

      <Tabs
        className="detail-tabs"
        activeKey={activeTab}
        onChange={setActiveTab}
        items={[
          {
            key: "overview",
            label: "概览",
            children: <OverviewPanel job={job} />,
          },
          {
            key: "logs",
            label: "实时日志",
            children: <LogsPanel job={job} />,
          },
          {
            key: "plist",
            label: "plist 配置",
            children: <PlistPanel job={job} />,
          },
        ]}
      />
    </>
  );
}

function InfoTile({ label, value }: { label: string; value: string }) {
  return (
    <div className="info-tile">
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}

function OverviewPanel({ job }: { job: LaunchdJob }) {
  return (
    <Space orientation="vertical" size={16} className="full-width overview-panel">
      <Descriptions bordered column={1} size="small">
        <Descriptions.Item label="运行计划">{scheduleSummary(job.schedule)}</Descriptions.Item>
        <Descriptions.Item label="命令">{commandSummary(job.execution)}</Descriptions.Item>
        <Descriptions.Item label="标识">
          <Typography.Text copyable className="path-line">
            {job.label}
          </Typography.Text>
        </Descriptions.Item>
        <Descriptions.Item label="plist 配置">
          <Typography.Text copyable className="path-line">
            {job.plistPath}
          </Typography.Text>
        </Descriptions.Item>
        <Descriptions.Item label="标准输出">
          <Typography.Text copyable className="path-line">
            {job.stdoutPath}
          </Typography.Text>
        </Descriptions.Item>
        <Descriptions.Item label="标准错误">
          <Typography.Text copyable className="path-line">
            {job.stderrPath}
          </Typography.Text>
        </Descriptions.Item>
        <Descriptions.Item label="更新时间">{new Date(job.lastModifiedAt).toLocaleString("zh-CN")}</Descriptions.Item>
      </Descriptions>
    </Space>
  );
}

export default App;
