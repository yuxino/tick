import { PlayCircleOutlined, ReloadOutlined, SearchOutlined } from "@ant-design/icons";
import { Button, Input, Switch, Tooltip } from "antd";
import { useMemo, useState } from "react";
import type { ScheduledJob } from "../types/scheduler";
import { commandSummary, scheduleSummary, statusLabel } from "../utils/scheduler";

interface JobsTableProps {
  jobs: ScheduledJob[];
  selectedId?: string;
  loading: boolean;
  busyId?: string;
  onSelect: (job: ScheduledJob) => void;
  onToggle: (job: ScheduledJob, enabled: boolean) => void;
  onRun: (job: ScheduledJob) => void;
  onRefresh: () => void;
}

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
    <section className="task-ledger" aria-label="任务列表">
      <header className="ledger-header">
        <div>
          <h2>任务</h2>
          <span className="ledger-count">{filteredJobs.length} 个</span>
        </div>
        <Tooltip title="刷新">
          <Button type="text" aria-label="刷新任务列表" icon={<ReloadOutlined />} loading={loading} onClick={onRefresh} />
        </Tooltip>
      </header>

      <div className="ledger-search">
        <Input
          allowClear
          prefix={<SearchOutlined />}
          placeholder="搜索任务、计划或命令"
          value={query}
          aria-label="搜索任务"
          onChange={(event) => setQuery(event.target.value)}
        />
      </div>

      {filteredJobs.length === 0 && !loading ? (
        <div className="ledger-empty">
          <span>—</span>
          <strong>{query ? "没有匹配的任务" : "还没有任务"}</strong>
          <p>{query ? "换一个名称、标识或命令关键词试试。" : "新建一个自动化后，它会出现在这里。"}</p>
        </div>
      ) : (
        <div className="ledger-list" role="listbox" aria-busy={loading} aria-label="任务账本">
          {filteredJobs.map((job, index) => {
            const selected = job.id === selectedId;
            const busy = busyId === job.id;

            return (
              <article key={job.id} className={`ledger-row ${selected ? "selected" : ""}`}>
                <button
                  type="button"
                  className="ledger-select"
                  role="option"
                  aria-selected={selected}
                  onClick={() => onSelect(job)}
                >
                  <span className="ledger-number">{String(index + 1).padStart(2, "0")}</span>
                  <span className={`ledger-status status-${job.status}`} title={statusLabel(job.status)} />
                  <span className="ledger-copy">
                    <strong>{job.name}</strong>
                    <span>{scheduleSummary(job.schedule)}</span>
                    <code>{job.label}</code>
                  </span>
                </button>

                <div className="ledger-row-actions">
                  <Switch
                    size="small"
                    checked={job.status === "enabled"}
                    loading={busy}
                    aria-label={`${job.name}启用状态`}
                    onChange={(checked) => onToggle(job, checked)}
                  />
                  <Tooltip title="立即运行">
                    <Button
                      type="text"
                      size="small"
                      icon={<PlayCircleOutlined />}
                      loading={busy}
                      aria-label={`立即运行${job.name}`}
                      onClick={() => onRun(job)}
                    />
                  </Tooltip>
                </div>
              </article>
            );
          })}
        </div>
      )}
    </section>
  );
}
