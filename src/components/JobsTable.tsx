import {
  PlayCircleOutlined,
  ReloadOutlined,
  SearchOutlined,
} from "@ant-design/icons";
import { Button, Empty, Input, Switch, Tag, Tooltip, Typography } from "antd";
import { useMemo, useState } from "react";
import type { LaunchdJob } from "../types/launchd";
import { commandSummary, scheduleSummary, statusLabel } from "../utils/launchd";

interface JobsTableProps {
  jobs: LaunchdJob[];
  selectedId?: string;
  loading: boolean;
  busyId?: string;
  onSelect: (job: LaunchdJob) => void;
  onEdit: (job: LaunchdJob) => void;
  onToggle: (job: LaunchdJob, enabled: boolean) => void;
  onRun: (job: LaunchdJob) => void;
  onDelete: (job: LaunchdJob) => void;
  onRefresh: () => void;
}

const statusColor: Record<LaunchdJob["status"], string> = {
  enabled: "green",
  disabled: "default",
  missing: "orange",
  error: "red",
};

export function JobsTable({
  jobs,
  selectedId,
  loading,
  busyId,
  onSelect,
  onToggle,
  onRun,
  onRefresh,
}: JobsTableProps) {
  const [query, setQuery] = useState("");
  const filteredJobs = useMemo(() => {
    const keyword = query.trim().toLowerCase();
    if (!keyword) return jobs;
    return jobs.filter((job) =>
      [job.name, job.description, job.label, scheduleSummary(job.schedule), commandSummary(job.execution)]
        .join(" ")
        .toLowerCase()
        .includes(keyword),
    );
  }, [jobs, query]);

  return (
    <div className="panel jobs-panel">
      <div className="panel-toolbar">
        <div>
          <Typography.Title level={4}>任务</Typography.Title>
          <Typography.Text type="secondary">{jobs.length} 个 LaunchAgent</Typography.Text>
        </div>
        <Tooltip title="刷新">
          <Button icon={<ReloadOutlined />} onClick={onRefresh} loading={loading} />
        </Tooltip>
      </div>
      <div className="job-search">
        <Input
          allowClear
          prefix={<SearchOutlined />}
          placeholder="搜索名称、命令或 label"
          value={query}
          onChange={(event) => setQuery(event.target.value)}
        />
      </div>

      {filteredJobs.length === 0 && !loading ? (
        <div className="jobs-empty">
          <Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description={query ? "没有匹配的任务" : "暂无任务"} />
        </div>
      ) : (
        <div className="job-list" aria-busy={loading}>
          {filteredJobs.map((job) => (
            <div
              key={job.id}
              className={`job-card ${job.id === selectedId ? "selected" : ""}`}
              role="button"
              tabIndex={0}
              onClick={() => onSelect(job)}
              onKeyDown={(event) => {
                if (event.key === "Enter" || event.key === " ") {
                  event.preventDefault();
                  onSelect(job);
                }
              }}
            >
              <div className="job-card-head">
                <div className="job-title-block">
                  <Typography.Text strong ellipsis>
                    {job.name}
                  </Typography.Text>
                  <Typography.Text type="secondary" ellipsis className="mono-subtle">
                    {job.label}
                  </Typography.Text>
                </div>
                <Tag color={statusColor[job.status]}>{statusLabel(job.status)}</Tag>
              </div>

              <div className="job-card-actions" onClick={(event) => event.stopPropagation()}>
                <span className="job-schedule">{scheduleSummary(job.schedule)}</span>
                <div className="job-card-controls">
                  <Switch
                    size="small"
                    checked={job.status === "enabled"}
                    loading={busyId === job.id}
                    onChange={(checked) => onToggle(job, checked)}
                  />
                  <Tooltip title="立即运行">
                    <Button size="small" type="text" icon={<PlayCircleOutlined />} loading={busyId === job.id} onClick={() => onRun(job)} />
                  </Tooltip>
                </div>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
