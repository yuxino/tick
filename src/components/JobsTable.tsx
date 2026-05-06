import {
  DeleteOutlined,
  EditOutlined,
  PlayCircleOutlined,
  ReloadOutlined,
  SearchOutlined,
} from "@ant-design/icons";
import { Button, Empty, Input, Popconfirm, Space, Switch, Tag, Tooltip, Typography } from "antd";
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
  onEdit,
  onToggle,
  onRun,
  onDelete,
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

              <div className="job-card-body">
                <div className="job-card-line">
                  <span>计划</span>
                  <strong>{scheduleSummary(job.schedule)}</strong>
                </div>
                <div className="job-card-line">
                  <span>命令</span>
                  <strong>{commandSummary(job.execution)}</strong>
                </div>
              </div>

              <div className="job-card-actions" onClick={(event) => event.stopPropagation()}>
                <Switch
                  size="small"
                  checked={job.status === "enabled"}
                  loading={busyId === job.id}
                  onChange={(checked) => onToggle(job, checked)}
                />
                <Space size={4} className="row-actions">
                  <Tooltip title="立即运行">
                    <Button size="small" icon={<PlayCircleOutlined />} loading={busyId === job.id} onClick={() => onRun(job)} />
                  </Tooltip>
                  <Tooltip title="编辑">
                    <Button size="small" icon={<EditOutlined />} onClick={() => onEdit(job)} />
                  </Tooltip>
                  <Popconfirm title="删除这个任务？" okText="删除" cancelText="取消" okButtonProps={{ danger: true }} onConfirm={() => onDelete(job)}>
                    <Button size="small" danger icon={<DeleteOutlined />} />
                  </Popconfirm>
                </Space>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
