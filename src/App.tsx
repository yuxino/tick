import {
  CalendarOutlined,
  ClockCircleOutlined,
  DeleteOutlined,
  EditOutlined,
  LeftOutlined,
  PlayCircleOutlined,
  PlusOutlined,
  ReloadOutlined,
  RightOutlined,
  SettingOutlined,
  UnorderedListOutlined,
} from "@ant-design/icons";
import { Alert, Button, Popconfirm, Switch, Tabs, Typography, message } from "antd";
import { useCallback, useEffect, useMemo, useState } from "react";
import tickMascot from "./assets/tick-mascot.png";
import { AutomationModal } from "./components/AutomationModal";
import { JobFormModal } from "./components/JobFormModal";
import { JobsTable } from "./components/JobsTable";
import { LogsPanel } from "./components/LogsPanel";
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
  const enabledCount = jobs.filter((job) => job.status === "enabled").length;
  const pausedCount = jobs.filter((job) => job.status === "disabled").length;
  const attentionCount = jobs.filter((job) => job.status === "missing" || job.status === "error").length;

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

  const viewCopy = activeView === "tasks"
    ? {
        eyebrow: "AUTOMATION LEDGER",
        title: "任务账本",
        description: `${enabledCount} 个任务正在按计划运行`,
      }
    : {
        eyebrow: "SCHEDULE INDEX",
        title: "运行日程",
        description: "用日历检查固定时间任务的分布",
      };

  return (
    <div className="app-shell">
      <div className="window-drag-region" data-tauri-drag-region />

      <AppRail
        activeView={activeView}
        total={jobs.length}
        enabled={enabledCount}
        paused={pausedCount}
        attention={attentionCount}
        onChangeView={setActiveView}
        onCreate={() => setAutomationOpen(true)}
        onSettings={() => setSettingsOpen(true)}
      />

      <main className="app-main">
        <header className="page-header">
          <div className="page-heading">
            <span className="eyebrow">{viewCopy.eyebrow}</span>
            <div className="page-title-row">
              <h1>{viewCopy.title}</h1>
              <span className="page-date">
                {new Intl.DateTimeFormat("zh-CN", { month: "long", day: "numeric", weekday: "long" }).format(new Date())}
              </span>
            </div>
            <p>{viewCopy.description}</p>
          </div>
          <div className="page-actions">
            <Button aria-label="刷新任务" icon={<ReloadOutlined />} loading={loading} onClick={loadJobs}>
              刷新
            </Button>
            <Button type="primary" icon={<PlusOutlined />} onClick={() => setAutomationOpen(true)}>
              新建任务
            </Button>
          </div>
        </header>

        {error && <Alert type="error" title={error} showIcon closable className="top-alert" onClose={() => setError(undefined)} />}

        <div className="page-stage">
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
            <div className="tasks-workbench">
              <JobsTable
                jobs={jobs}
                selectedId={selectedJob?.id}
                loading={loading}
                busyId={busyId}
                onSelect={(job) => setSelectedId(job.id)}
                onRefresh={loadJobs}
                onToggle={handleToggle}
                onRun={handleRun}
              />

              <div className="task-inspector">
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
        </div>
      </main>

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
    </div>
  );
}

function AppRail({
  activeView,
  total,
  enabled,
  paused,
  attention,
  onChangeView,
  onCreate,
  onSettings,
}: {
  activeView: MainView;
  total: number;
  enabled: number;
  paused: number;
  attention: number;
  onChangeView: (view: MainView) => void;
  onCreate: () => void;
  onSettings: () => void;
}) {
  return (
    <aside className="control-rail">
      <div className="rail-brand" aria-label="Tick">
        <span className="rail-logo-frame" aria-hidden="true">
          <img src={tickMascot} alt="" />
        </span>
        <div>
          <strong>tick</strong>
          <span>时间调度器</span>
        </div>
      </div>

      <nav className="rail-navigation" aria-label="主导航">
        <span className="rail-section-label">工作区</span>
        <button
          type="button"
          className={activeView === "tasks" ? "active" : ""}
          aria-label="任务"
          aria-current={activeView === "tasks" ? "page" : undefined}
          onClick={() => onChangeView("tasks")}
        >
          <UnorderedListOutlined />
          <span>任务</span>
          <b>{total}</b>
        </button>
        <button
          type="button"
          className={activeView === "schedule" ? "active" : ""}
          aria-label="日程"
          aria-current={activeView === "schedule" ? "page" : undefined}
          onClick={() => onChangeView("schedule")}
        >
          <CalendarOutlined />
          <span>日程</span>
        </button>
      </nav>

      <div className="rail-index" aria-label="任务状态摘要">
        <span className="rail-section-label">运行索引</span>
        <div><i className="index-dot enabled" />正在运行 <strong>{enabled}</strong></div>
        <div><i className="index-dot paused" />已经暂停 <strong>{paused}</strong></div>
        <div><i className="index-dot attention" />需要处理 <strong>{attention}</strong></div>
      </div>

      <div className="rail-footer">
        <Button className="rail-create" type="primary" icon={<PlusOutlined />} aria-label="新建任务" onClick={onCreate}>
          新建任务
        </Button>
        <button type="button" className="rail-settings" aria-label="设置" onClick={onSettings}>
          <SettingOutlined />
          <span>设置</span>
        </button>
      </div>
    </aside>
  );
}

function EmptyTasks({ onCreate, onManual }: { onCreate: () => void; onManual: () => void }) {
  return (
    <section className="empty-tasks">
      <div className="empty-illustration" aria-hidden="true">
        <img src={tickMascot} alt="" />
        <span>00:00</span>
      </div>
      <span className="eyebrow">YOUR FIRST AUTOMATION</span>
      <h2>让 Mac 按时替你做事</h2>
      <p>描述一件需要重复完成的事情，或直接填写运行时间与脚本。保存前你仍然可以检查和试跑。</p>
      <div className="empty-actions">
        <Button type="primary" icon={<PlusOutlined />} onClick={onCreate}>
          描述一个任务
        </Button>
        <Button onClick={onManual}>手动填写</Button>
      </div>
    </section>
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
  const intervalJobs = jobs.filter((job) => job.status === "enabled" && job.schedule.mode === "interval").length;

  function moveMonth(offset: number) {
    const next = parseMonthKey(visibleMonth);
    next.setMonth(next.getMonth() + offset);
    setVisibleMonth(monthKey(next));
  }

  return (
    <section className="schedule-shell">
      <div className="planner-toolbar">
        <div>
          <span className="eyebrow">MONTHLY FIELD</span>
          <h2>{monthDate.toLocaleDateString("zh-CN", { year: "numeric", month: "long" })}</h2>
        </div>
        <div className="planner-context">
          <span><strong>{activeJobs}</strong> 个启用任务</span>
          {intervalJobs > 0 ? <span><strong>{intervalJobs}</strong> 个持续重复任务不按次数统计</span> : null}
        </div>
        <div className="planner-actions">
          <Button icon={<LeftOutlined />} onClick={() => moveMonth(-1)} aria-label="上个月" />
          <Button onClick={() => setVisibleMonth(monthKey(new Date()))}>今天</Button>
          <Button icon={<RightOutlined />} onClick={() => moveMonth(1)} aria-label="下个月" />
        </div>
      </div>

      <div className="calendar-weekdays" aria-hidden="true">
        {["周一", "周二", "周三", "周四", "周五", "周六", "周日"].map((day) => (
          <span key={day}>{day}</span>
        ))}
      </div>

      <div className="calendar-grid" aria-busy={loading}>
        {days.map((day) => {
          const key = dateKey(day);
          const dayJobs = jobsForDate(jobs, day).filter((job) => job.schedule.mode === "calendar");
          const visibleJobs = dayJobs.slice(0, 3);
          const isMuted = monthKey(day) !== visibleMonth;
          const isToday = key === todayKey;

          return (
            <div key={key} className={`calendar-day ${isMuted ? "muted" : ""} ${isToday ? "today" : ""}`}>
              <div className="calendar-day-head">
                <strong>{padNumber(day.getDate())}</strong>
                {isToday ? <span>今天</span> : null}
              </div>

              <div className="calendar-events">
                {visibleJobs.length > 0 ? (
                  visibleJobs.map((job) => (
                    <button
                      key={job.id}
                      type="button"
                      className={`calendar-event status-${job.status}`}
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
                  <span className="calendar-empty">—</span>
                )}
                {dayJobs.length > visibleJobs.length ? <span className="calendar-more">+{dayJobs.length - visibleJobs.length}</span> : null}
              </div>
            </div>
          );
        })}
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
    <section className="detail-panel">
      <header className="detail-identity">
        <div className="detail-status-line">
          <span className={`status-pill status-${job.status}`}>
            <i />{statusLabel(job.status)}
          </span>
          <code>{job.label}</code>
        </div>
        <h2>{job.name}</h2>
        <p>{job.description || "没有补充说明"}</p>
      </header>

      <div className="detail-specimens">
        <section className="schedule-specimen">
          <span className="specimen-label">运行时刻</span>
          <strong>{runTimeLabel(job)}</strong>
          <small>{scheduleSummary(job.schedule)}</small>
        </section>
        <section className="command-specimen">
          <span className="specimen-label">执行内容</span>
          <code>{commandSummary(job.execution)}</code>
        </section>
      </div>

      <div className="detail-actions">
        <label className="enable-control">
          <Switch
            checked={job.status === "enabled"}
            loading={busy}
            aria-label={`${job.name}启用状态`}
            onChange={(checked) => onToggle(job, checked)}
          />
          <span>{job.status === "enabled" ? "已加入日程" : "当前已暂停"}</span>
        </label>
        <div>
          <Button type="primary" icon={<PlayCircleOutlined />} loading={busy} onClick={() => onRun(job)}>
            立即运行
          </Button>
          <Button icon={<EditOutlined />} onClick={() => onEdit(job)}>
            编辑
          </Button>
          <Popconfirm
            title="删除这个任务？"
            description="对应的 LaunchAgent 配置也会一并移除。"
            okText="删除"
            cancelText="取消"
            okButtonProps={{ danger: true }}
            onConfirm={() => onDelete(job)}
          >
            <Button className="delete-action" danger icon={<DeleteOutlined />} loading={busy} aria-label="删除任务" />
          </Popconfirm>
        </div>
      </div>

      <Tabs
        className="detail-tabs"
        activeKey={activeTab}
        onChange={setActiveTab}
        items={[
          {
            key: "overview",
            label: "任务定义",
            children: <OverviewPanel job={job} />,
          },
          {
            key: "logs",
            label: "实时日志",
            children: <LogsPanel job={job} />,
          },
          {
            key: "plist",
            label: "plist",
            children: <PlistPanel job={job} />,
          },
        ]}
      />
    </section>
  );
}

function OverviewPanel({ job }: { job: LaunchdJob }) {
  return (
    <dl className="metadata-ledger">
      <MetadataRow label="运行计划" value={scheduleSummary(job.schedule)} />
      <MetadataRow label="执行命令" value={commandSummary(job.execution)} mono />
      <MetadataRow label="任务标识" value={job.label} mono copyable />
      <MetadataRow label="plist 配置" value={displayPath(job.plistPath)} copyText={job.plistPath} mono />
      <MetadataRow label="标准输出" value={displayPath(job.stdoutPath)} copyText={job.stdoutPath} mono />
      <MetadataRow label="标准错误" value={displayPath(job.stderrPath)} copyText={job.stderrPath} mono />
      <MetadataRow label="配置更新" value={new Date(job.lastModifiedAt).toLocaleString("zh-CN")} />
    </dl>
  );
}

function MetadataRow({
  label,
  value,
  mono = false,
  copyable = false,
  copyText,
}: {
  label: string;
  value: string;
  mono?: boolean;
  copyable?: boolean;
  copyText?: string;
}) {
  return (
    <div>
      <dt>{label}</dt>
      <dd className={mono ? "mono-value" : undefined} title={copyText ?? value}>
        {copyable || copyText ? (
          <Typography.Text copyable={{ text: copyText ?? value }}>{value}</Typography.Text>
        ) : value}
      </dd>
    </div>
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
  if (job.status === "missing" || job.status === "error") return false;
  if (job.schedule.mode === "interval") return true;

  const { month, day } = job.schedule.calendar;
  const currentMonth = date.getMonth() + 1;
  const currentDay = date.getDate();

  if (month && month !== currentMonth) return false;
  if (day && day !== currentDay) return false;
  return true;
}

function runTimeLabel(job: LaunchdJob) {
  if (job.schedule.mode === "interval") {
    return formatInterval(job.schedule.interval.seconds);
  }

  const { hour = 0, minute = 0, second = 0 } = job.schedule.calendar;
  return `${padNumber(hour)}:${padNumber(minute)}${second ? `:${padNumber(second)}` : ""}`;
}

function formatInterval(seconds: number) {
  if (seconds % 86400 === 0) return `每 ${seconds / 86400} 天`;
  if (seconds % 3600 === 0) return `每 ${seconds / 3600} 小时`;
  if (seconds % 60 === 0) return `每 ${seconds / 60} 分钟`;
  return `每 ${seconds} 秒`;
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
