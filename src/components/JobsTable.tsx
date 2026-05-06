import {
  DeleteOutlined,
  EditOutlined,
  PlayCircleOutlined,
  ReloadOutlined,
} from "@ant-design/icons";
import { Button, Popconfirm, Space, Switch, Table, Tag, Tooltip, Typography } from "antd";
import type { ColumnsType } from "antd/es/table";
import type { LaunchdJob } from "../types/launchd";
import { commandSummary, scheduleSummary } from "../utils/launchd";

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
  const columns: ColumnsType<LaunchdJob> = [
    {
      title: "Job",
      dataIndex: "name",
      render: (_, job) => (
        <Space direction="vertical" size={0}>
          <Typography.Text strong>{job.name}</Typography.Text>
          <Typography.Text type="secondary" ellipsis className="mono-subtle">
            {job.label}
          </Typography.Text>
        </Space>
      ),
    },
    {
      title: "Status",
      dataIndex: "status",
      width: 110,
      render: (status: LaunchdJob["status"]) => <Tag color={statusColor[status]}>{status}</Tag>,
    },
    {
      title: "Schedule",
      render: (_, job) => scheduleSummary(job.schedule),
    },
    {
      title: "Command",
      render: (_, job) => (
        <Typography.Text ellipsis className="command-cell">
          {commandSummary(job.execution)}
        </Typography.Text>
      ),
    },
    {
      title: "Actions",
      width: 190,
      render: (_, job) => (
        <Space size={4} className="row-actions">
          <Switch
            size="small"
            checked={job.status === "enabled"}
            loading={busyId === job.id}
            onChange={(checked) => onToggle(job, checked)}
          />
          <Tooltip title="Run now">
            <Button size="small" icon={<PlayCircleOutlined />} loading={busyId === job.id} onClick={() => onRun(job)} />
          </Tooltip>
          <Tooltip title="Edit">
            <Button size="small" icon={<EditOutlined />} onClick={() => onEdit(job)} />
          </Tooltip>
          <Popconfirm title="Delete this job?" okText="Delete" okButtonProps={{ danger: true }} onConfirm={() => onDelete(job)}>
            <Button size="small" danger icon={<DeleteOutlined />} />
          </Popconfirm>
        </Space>
      ),
    },
  ];

  return (
    <div className="panel">
      <div className="panel-toolbar">
        <Typography.Title level={4}>Jobs</Typography.Title>
        <Button icon={<ReloadOutlined />} onClick={onRefresh} loading={loading} />
      </div>
      <Table
        rowKey="id"
        size="middle"
        loading={loading}
        columns={columns}
        dataSource={jobs}
        pagination={false}
        rowClassName={(job) => (job.id === selectedId ? "selected-row" : "")}
        onRow={(job) => ({ onClick: () => onSelect(job) })}
        scroll={{ x: 760 }}
      />
    </div>
  );
}
