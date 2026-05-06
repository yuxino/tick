import { PlusOutlined } from "@ant-design/icons";
import { Alert, Button, Descriptions, Empty, Layout, message, Space, Tabs, Tag, Typography } from "antd";
import { useCallback, useEffect, useMemo, useState } from "react";
import { JobFormModal } from "./components/JobFormModal";
import { JobsTable } from "./components/JobsTable";
import { LogsPanel } from "./components/LogsPanel";
import { PlistPanel } from "./components/PlistPanel";
import {
  deleteLaunchdJob,
  disableLaunchdJob,
  enableLaunchdJob,
  listLaunchdJobs,
  runLaunchdJobNow,
  saveLaunchdJob,
} from "./services/launchd";
import type { LaunchdJob, LaunchdJobInput } from "./types/launchd";
import { commandSummary, emptyJobInput, scheduleSummary, toJobInput } from "./utils/launchd";

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
      setError(String(err));
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
      message.success("Job saved");
      setFormOpen(false);
      setEditingJob(undefined);
      await loadJobs();
      setSelectedId(saved.id);
    } catch (err) {
      message.error(String(err));
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
      message.error(String(err));
    } finally {
      setBusyId(undefined);
    }
  }

  return (
    <Layout className="app-shell">
      <Header className="app-header">
        <div>
          <div className="app-title">Tick</div>
          <div className="app-subtitle">LaunchAgent scheduler</div>
        </div>
        <Space>
          <Button icon={<PlusOutlined />} type="primary" onClick={openCreate}>
            New Job
          </Button>
        </Space>
      </Header>

      <Content className="app-content">
        {error && <Alert type="error" message={error} showIcon className="top-alert" />}

        {jobs.length === 0 && !loading ? (
          <div className="empty-page">
            <Empty description="No launchd jobs yet">
              <Button icon={<PlusOutlined />} type="primary" onClick={openCreate}>
                Create Job
              </Button>
            </Empty>
          </div>
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
              onToggle={(job, enabled) =>
                withBusy(
                  job,
                  async () => {
                    if (enabled) {
                      await enableLaunchdJob(job.id);
                    } else {
                      await disableLaunchdJob(job.id);
                    }
                  },
                  enabled ? "Job enabled" : "Job disabled",
                )
              }
              onRun={(job) => withBusy(job, () => runLaunchdJobNow(job.id), "Job started")}
              onDelete={(job) => withBusy(job, () => deleteLaunchdJob(job.id), "Job deleted")}
            />

            <div className="panel detail-panel">
              {selectedJob ? (
                <Tabs
                  defaultActiveKey="overview"
                  items={[
                    {
                      key: "overview",
                      label: "Overview",
                      children: <OverviewPanel job={selectedJob} />,
                    },
                    {
                      key: "logs",
                      label: "Logs",
                      children: <LogsPanel job={selectedJob} />,
                    },
                    {
                      key: "plist",
                      label: "Plist",
                      children: <PlistPanel job={selectedJob} />,
                    },
                  ]}
                />
              ) : (
                <div className="empty-detail">Select a job to inspect it.</div>
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
    </Layout>
  );
}

function OverviewPanel({ job }: { job: LaunchdJob }) {
  return (
    <Space direction="vertical" size={16} className="full-width">
      <div className="overview-header">
        <div>
          <Typography.Title level={3}>{job.name}</Typography.Title>
          <Typography.Text type="secondary">{job.description || "No description"}</Typography.Text>
        </div>
        <Tag color={job.status === "enabled" ? "green" : job.status === "missing" ? "orange" : "default"}>
          {job.status}
        </Tag>
      </div>

      <Descriptions bordered column={1} size="small">
        <Descriptions.Item label="Schedule">{scheduleSummary(job.schedule)}</Descriptions.Item>
        <Descriptions.Item label="Command">{commandSummary(job.execution)}</Descriptions.Item>
        <Descriptions.Item label="Label">
          <Typography.Text copyable className="path-line">
            {job.label}
          </Typography.Text>
        </Descriptions.Item>
        <Descriptions.Item label="Plist">
          <Typography.Text copyable className="path-line">
            {job.plistPath}
          </Typography.Text>
        </Descriptions.Item>
        <Descriptions.Item label="stdout">
          <Typography.Text copyable className="path-line">
            {job.stdoutPath}
          </Typography.Text>
        </Descriptions.Item>
        <Descriptions.Item label="stderr">
          <Typography.Text copyable className="path-line">
            {job.stderrPath}
          </Typography.Text>
        </Descriptions.Item>
        <Descriptions.Item label="Updated">{new Date(job.lastModifiedAt).toLocaleString()}</Descriptions.Item>
      </Descriptions>
    </Space>
  );
}

export default App;
