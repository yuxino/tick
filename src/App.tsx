import {
  CalendarOutlined,
  ClockCircleOutlined,
  DeleteOutlined,
  EditOutlined,
  LeftOutlined,
  PlayCircleOutlined,
  PlusOutlined,
  RightOutlined,
  SettingOutlined,
  UnorderedListOutlined,
} from "@ant-design/icons";
import { Alert, Button, Descriptions, Layout, message, Popconfirm, Space, Switch, Tabs, Tag, Typography } from "antd";
import { useCallback, useEffect, useMemo, useState } from "react";
import { JobFormModal } from "./components/JobFormModal";
import { AutomationModal } from "./components/AutomationModal";
import { JobsTable } from "./components/JobsTable";
import { LogsPanel } from "./components/LogsPanel";
import tickMascot from "./assets/tick-mascot.png";
import { PlistPanel } from "./components/PlistPanel";
import { SettingsModal } from "./components/SettingsModal";
import {
  deleteLaunchdJob,
  disableLaunchdJob,
  enableLaunchdJob,
  listLaunchdJobs,
  runLaunchdJobNow,
  saveLaunchdJob,
} from "./services/launchd";
import type { AutomationDraft } from "./services/launchd";
import type { LaunchdJob, LaunchdJobInput } from "./types/launchd";
import { friendlyError } from "./utils/errors";
import { commandSummary, emptyJobInput, scheduleSummary, statusLabel, toJobInput } from "./utils/launchd";
import { displayPath } from "./utils/paths";

const { Header, Content } = Layout;
type MainView = "tasks" | "schedule";

function App() {
  const [jobs, setJobs] = useState<LaunchdJob[]>([]);
  const [selectedId, setSelectedId] = useState<string>();
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [busyId, setBusyId] = useState<string>();
  const [error, setError] = useState<string>();
  const [formOpen, setFormOpen] = useState(false);
  const [editingJob, setEditingJob] = useState<LaunchdJob>();
  const [activeView, setActiveView] = useState<MainView>("tasks");
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [automationOpen, setAutomationOpen] = useState(false);
  const [automationDraft, setAutomationDraft] = useState<AutomationDraft>();

  const selectedJob = useMemo(
    () => jobs.find((job) => job.id === selectedId) ?? jobs[0],
    [jobs, selectedId],
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
    setAutomationDraft(undefined);
    setFormOpen(true);
  }

  function openEdit(job: LaunchdJob) {
    setEditingJob(job);
    setAutomationDraft(undefined);
    setFormOpen(true);
  }

  function openGeneratedAutomation(draft: AutomationDraft) {
    setAutomationDraft(draft);
    setEditingJob(undefined);
    setAutomationOpen(false);
    setFormOpen(true);
  }

  async function handleSave(input: LaunchdJobInput) {
    setSaving(true);
    try {
      const saved = await saveLaunchdJob(input);
      message.success("任务已保存");
      setFormOpen(false);
      setEditingJob(undefined);
      setAutomationDraft(undefined);
      await loadJobs();
      setSelectedId(saved.id);
    } catch (err) {
      message.error(friendlyError(err));
    } finally {
      setSaving(false);
    }
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
          <img className="brand-mark" src={tickMascot} alt="" />
          <div className="app-title">Tick</div>
        </div>
        <div className="header-controls">
          <div className="view-switch" role="tablist" aria-label="主视图">
            <button
              type="button"
              className={activeView === "tasks" ? "active" : ""}
              role="tab"
              aria-selected={activeView === "tasks"}
              onClick={() => setActiveView("tasks")}
            >
              <UnorderedListOutlined />
              任务
            </button>
            <button
              type="button"
              className={activeView === "schedule" ? "active" : ""}
              role="tab"
              aria-selected={activeView === "schedule"}
              onClick={() => setActiveView("schedule")}
            >
              <CalendarOutlined />
              日程
            </button>
          </div>
          <Space className="header-actions">
            <Button aria-label="设置" icon={<SettingOutlined />} onClick={() => setSettingsOpen(true)} />
            <Button icon={<PlusOutlined />} type="primary" onClick={() => setAutomationOpen(true)}>
              新建任务
            </Button>
          </Space>
        </div>
      </Header>

      <Content className="app-content">
        {error && <Alert type="error" title={error} showIcon className="top-alert" />}

        {activeView === "schedule" ? (
          <ScheduleCalendar
            jobs={jobs}
            loading={loading}
            onOpenJob={(job) => {
              setSelectedId(job.id);
              setActiveView("tasks");
            }}
          />
        ) : jobs.length === 0 && !loading ? (
          <EmptyTasks onCreate={() => setAutomationOpen(true)} onManual={openCreate} />
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
        initialValue={editingJob ? toJobInput(editingJob) : automationDraft?.job ?? emptyJobInput()}
        saving={saving}
        draftSummary={automationDraft?.summary}
        draftRisks={automationDraft?.risks}
        onCancel={() => {
          setFormOpen(false);
          setEditingJob(undefined);
          setAutomationDraft(undefined);
        }}
        onSubmit={handleSave}
      />
      <AutomationModal
        open={automationOpen}
        onCancel={() => setAutomationOpen(false)}
        onManual={() => {
          setAutomationOpen(false);
          openCreate();
        }}
        onGenerated={openGeneratedAutomation}
      />
      <SettingsModal open={settingsOpen} onClose={() => setSettingsOpen(false)} />
    </Layout>
  );
}

function EmptyTasks({ onCreate, onManual }: { onCreate: () => void; onManual: () => void }) {
  return (
    <div className="empty-tasks">
      <img src={tickMascot} alt="" />
      <Typography.Title level={3}>还没有任务</Typography.Title>
      <Typography.Paragraph type="secondary">创建一个定时任务，Tick 会负责保存配置和运行日志。</Typography.Paragraph>
      <Space>
        <Button type="primary" icon={<PlusOutlined />} onClick={onCreate}>
          新建任务
        </Button>
        <Button onClick={onManual}>手动填写</Button>
      </Space>
    </div>
  );
}

function ScheduleCalendar({
  jobs,
  loading,
  onOpenJob,
}: {
  jobs: LaunchdJob[];
  loading: boolean;
  onOpenJob: (job: LaunchdJob) => void;
}) {
  const [visibleMonth, setVisibleMonth] = useState(() => monthKey(new Date()));
  const todayKey = dateKey(new Date());
  const days = useMemo(() => monthGrid(visibleMonth), [visibleMonth]);
  const monthDate = parseMonthKey(visibleMonth);
  const activeJobs = jobs.filter((job) => job.status === "enabled").length;
  const monthRuns = days
    .filter((day) => monthKey(day) === visibleMonth)
    .reduce((sum, day) => sum + jobsForDate(jobs, day).filter((job) => job.status === "enabled").length, 0);

  function moveMonth(offset: number) {
    const next = parseMonthKey(visibleMonth);
    next.setMonth(next.getMonth() + offset);
    setVisibleMonth(monthKey(next));
  }

  return (
    <section className="schedule-shell">
      <div className="schedule-board panel">
        <div className="schedule-toolbar">
          <div className="schedule-heading">
            <Typography.Text type="secondary">日程视图</Typography.Text>
            <Typography.Title level={3}>{monthDate.toLocaleDateString("zh-CN", { year: "numeric", month: "long" })}</Typography.Title>
          </div>
          <div className="schedule-toolbar-actions">
            <Button icon={<LeftOutlined />} onClick={() => moveMonth(-1)} aria-label="上个月" />
            <Button onClick={() => setVisibleMonth(monthKey(new Date()))}>今天</Button>
            <Button icon={<RightOutlined />} onClick={() => moveMonth(1)} aria-label="下个月" />
          </div>
        </div>

        <div className="schedule-summary-row">
          <InfoTile label="启用任务" value={`${activeJobs} 个`} />
          <InfoTile label="本月预计运行" value={`${monthRuns} 次`} />
          <InfoTile label="可见范围" value="按天聚合" />
        </div>

        <div className="calendar-weekdays" aria-hidden="true">
          {["周一", "周二", "周三", "周四", "周五", "周六", "周日"].map((day) => (
            <span key={day}>{day}</span>
          ))}
        </div>

        <div className="calendar-grid" aria-busy={loading}>
          {days.map((day) => {
            const key = dateKey(day);
            const dayJobs = jobsForDate(jobs, day);
            const visibleJobs = dayJobs.slice(0, 4);
            const isMuted = monthKey(day) !== visibleMonth;
            const isToday = key === todayKey;

            return (
              <div key={key} className={`calendar-day ${isMuted ? "muted" : ""} ${isToday ? "today" : ""}`}>
                <div className="calendar-day-head">
                  <strong>{day.getDate()}</strong>
                  {isToday ? <Tag color="green">今天</Tag> : null}
                </div>

                <div className="calendar-events">
                  {visibleJobs.length > 0 ? (
                    visibleJobs.map((job) => (
                      <button
                        key={job.id}
                        type="button"
                        className={`calendar-event ${job.status === "enabled" ? "enabled" : "disabled"}`}
                        onClick={() => onOpenJob(job)}
                      >
                        <span className="calendar-event-time">
                          <ClockCircleOutlined />
                          {runTimeLabel(job)}
                        </span>
                        <span className="calendar-event-name">{job.name}</span>
                      </button>
                    ))
                  ) : (
                    <span className="calendar-empty">无运行</span>
                  )}
                  {dayJobs.length > visibleJobs.length ? <span className="calendar-more">+{dayJobs.length - visibleJobs.length}</span> : null}
                </div>
              </div>
            );
          })}
        </div>
      </div>
    </section>
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
          <Typography.Text copyable={{ text: job.plistPath }} className="path-line" title={job.plistPath}>
            {displayPath(job.plistPath)}
          </Typography.Text>
        </Descriptions.Item>
        <Descriptions.Item label="标准输出">
          <Typography.Text copyable={{ text: job.stdoutPath }} className="path-line" title={job.stdoutPath}>
            {displayPath(job.stdoutPath)}
          </Typography.Text>
        </Descriptions.Item>
        <Descriptions.Item label="标准错误">
          <Typography.Text copyable={{ text: job.stderrPath }} className="path-line" title={job.stderrPath}>
            {displayPath(job.stderrPath)}
          </Typography.Text>
        </Descriptions.Item>
        <Descriptions.Item label="更新时间">{new Date(job.lastModifiedAt).toLocaleString("zh-CN")}</Descriptions.Item>
      </Descriptions>
    </Space>
  );
}

function jobsForDate(jobs: LaunchdJob[], date: Date) {
  return jobs
    .filter((job) => jobMatchesDate(job, date))
    .sort((a, b) => {
      if (a.status !== b.status) return a.status === "enabled" ? -1 : 1;
      return runSortValue(a) - runSortValue(b);
    });
}

function jobMatchesDate(job: LaunchdJob, date: Date) {
  if (job.status === "missing" || job.status === "error") {
    return false;
  }

  if (job.schedule.mode === "interval") {
    return true;
  }

  const { month, day } = job.schedule.calendar;
  const currentMonth = date.getMonth() + 1;
  const currentDay = date.getDate();

  if (month && month !== currentMonth) return false;
  if (day && day !== currentDay) return false;
  return true;
}

function runTimeLabel(job: LaunchdJob) {
  if (job.schedule.mode === "interval") {
    return `每 ${job.schedule.interval.seconds} 秒`;
  }

  const { hour = 0, minute = 0, second = 0 } = job.schedule.calendar;
  return `${padNumber(hour)}:${padNumber(minute)}:${padNumber(second)}`;
}

function runSortValue(job: LaunchdJob) {
  if (job.schedule.mode === "interval") return -1;
  const { hour = 0, minute = 0, second = 0 } = job.schedule.calendar;
  return hour * 3600 + minute * 60 + second;
}

function monthGrid(value: string) {
  const first = parseMonthKey(value);
  const mondayOffset = (first.getDay() + 6) % 7;
  const start = addDays(first, -mondayOffset);
  return Array.from({ length: 42 }, (_, index) => addDays(start, index));
}

function parseMonthKey(value: string) {
  const [year, month] = value.split("-").map(Number);
  return new Date(year, (month || 1) - 1, 1);
}

function addDays(date: Date, days: number) {
  const next = new Date(date);
  next.setDate(next.getDate() + days);
  return next;
}

function monthKey(date: Date) {
  return `${date.getFullYear()}-${padNumber(date.getMonth() + 1)}`;
}

function dateKey(date: Date) {
  return `${monthKey(date)}-${padNumber(date.getDate())}`;
}

function padNumber(value: number) {
  return String(value).padStart(2, "0");
}

export default App;
