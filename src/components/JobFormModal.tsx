import { DeleteOutlined, PlayCircleOutlined, PlusOutlined, RobotOutlined } from "@ant-design/icons";
import CodeMirror from "@uiw/react-codemirror";
import { javascript } from "@codemirror/lang-javascript";
import type { Extension } from "@codemirror/state";
import { Button, Collapse, Form, Input, InputNumber, message, Modal, Segmented, Space, TimePicker, Typography } from "antd";
import type { FormInstance } from "antd";
import dayjs from "dayjs";
import { useEffect, useMemo, useState } from "react";
import type { ExecutionMode, LaunchdExecution, LaunchdJobInput, LaunchdSchedule } from "../types/launchd";
import { generateNodeScript, runNodeScriptDebug } from "../services/launchd";
import type { RunNodeScriptDebugResponse } from "../services/launchd";
import { ScriptDebugPanel } from "./ScriptDebugPanel";
import { tickEditorTheme } from "../editorTheme";
import { friendlyError } from "../utils/errors";
import { defaultExecution, defaultSchedule, emptyJobInput } from "../utils/launchd";

interface JobFormModalProps {
  open: boolean;
  initialValue?: LaunchdJobInput;
  saving: boolean;
  onCancel: () => void;
  onSubmit: (input: LaunchdJobInput) => Promise<void>;
}

type SchedulePreset = "daily" | "monthly" | "yearly" | "interval";

export function JobFormModal({ open, initialValue, saving, onCancel, onSubmit }: JobFormModalProps) {
  const [form] = Form.useForm<LaunchdJobInput>();
  const [schedulePreset, setSchedulePreset] = useState<SchedulePreset>("daily");
  const executionMode = Form.useWatch(["execution", "mode"], form) as ExecutionMode | undefined;
  const schedule = Form.useWatch("schedule", form) as LaunchdSchedule | undefined;
  const execution = Form.useWatch("execution", form) as LaunchdExecution | undefined;
  const inlineScript = Form.useWatch(["execution", "inlineScript"], form) as string | undefined;
  const workingDirectory = Form.useWatch(["execution", "workingDirectory"], form) as string | undefined;

  const extensions = useMemo(() => [tickEditorTheme, javascript({ jsx: true, typescript: true })], []);

  useEffect(() => {
    if (open) {
      const nextValue = initialValue ?? emptyJobInput();
      form.setFieldsValue(nextValue);
      setSchedulePreset(detectSchedulePreset(nextValue.schedule));
    }
  }, [form, initialValue, open]);

  async function handleOk() {
    await form.validateFields();
    const values = form.getFieldsValue(true) as LaunchdJobInput;
    await onSubmit(values);
  }

  const previewSchedule = schedule ?? defaultSchedule;
  const previewExecution = execution ?? defaultExecution;
  const currentExecutionMode = (executionMode ?? previewExecution.mode ?? "inline_shell") as ExecutionMode;

  function getSchedule() {
    return form.getFieldValue("schedule") ?? defaultSchedule;
  }

  function updateSchedule(nextSchedule: LaunchdSchedule) {
    form.setFieldsValue({ schedule: nextSchedule });
  }

  function updateCalendar(patch: Partial<LaunchdSchedule["calendar"]>) {
    const current = getSchedule();
    updateSchedule({
      ...current,
      mode: "calendar",
      calendar: {
        ...current.calendar,
        ...patch,
      },
      interval: current.interval ?? defaultSchedule.interval,
    });
  }

  function changeSchedulePreset(nextPreset: SchedulePreset) {
    setSchedulePreset(nextPreset);
    const current = getSchedule();
    const now = new Date();

    if (nextPreset === "interval") {
      updateSchedule({
        ...current,
        mode: "interval",
        interval: current.interval?.seconds ? current.interval : defaultSchedule.interval,
      });
      return;
    }

    const baseCalendar = {
      ...defaultSchedule.calendar,
      ...current.calendar,
    };

    if (nextPreset === "daily") {
      updateSchedule({
        ...current,
        mode: "calendar",
        calendar: {
          ...baseCalendar,
          month: undefined,
          day: undefined,
        },
      });
      return;
    }

    if (nextPreset === "monthly") {
      updateSchedule({
        ...current,
        mode: "calendar",
        calendar: {
          ...baseCalendar,
          month: undefined,
          day: baseCalendar.day ?? now.getDate(),
        },
      });
      return;
    }

    updateSchedule({
      ...current,
      mode: "calendar",
      calendar: {
        ...baseCalendar,
        month: baseCalendar.month ?? now.getMonth() + 1,
        day: baseCalendar.day ?? now.getDate(),
      },
    });
  }

  function updateTime(value: string) {
    const [hour = "0", minute = "0", second = "0"] = value.split(":");
    updateCalendar({
      hour: Number(hour),
      minute: Number(minute),
      second: Number(second),
    });
  }

  function updateYearlyDate(value: string) {
    const [, month, day] = value.split("-");
    if (!month || !day) return;
    updateCalendar({
      month: Number(month),
      day: Number(day),
    });
  }

  function applyInlineScript(script: string) {
    form.setFieldsValue({
      execution: {
        ...previewExecution,
        mode: "inline_shell",
        inlineScript: script,
        interpreter: "/usr/bin/env node",
      },
    });
  }

  function changeExecutionMode(nextMode: ExecutionMode) {
    form.setFieldsValue({
      execution: {
        ...previewExecution,
        mode: nextMode,
        interpreter: nextMode === "inline_shell" ? "/usr/bin/env node" : previewExecution.interpreter || "/usr/bin/env node",
      },
    });
  }

  return (
    <Modal
      title={initialValue?.id ? "编辑任务" : "新建任务"}
      open={open}
      width={900}
      onCancel={onCancel}
      onOk={handleOk}
      confirmLoading={saving}
      okText="保存"
      cancelText="取消"
      destroyOnHidden
    >
      <Form form={form} layout="vertical" initialValues={emptyJobInput()} className="job-form">
        <Form.Item name="id" hidden>
          <Input />
        </Form.Item>
        <Form.Item name={["schedule", "mode"]} hidden>
          <Input />
        </Form.Item>
        <Form.Item name={["schedule", "calendar", "month"]} hidden>
          <InputNumber />
        </Form.Item>
        <Form.Item name={["schedule", "calendar", "day"]} hidden>
          <InputNumber />
        </Form.Item>
        <Form.Item name={["schedule", "calendar", "hour"]} hidden>
          <InputNumber />
        </Form.Item>
        <Form.Item name={["schedule", "calendar", "minute"]} hidden>
          <InputNumber />
        </Form.Item>
        <Form.Item name={["schedule", "calendar", "second"]} hidden>
          <InputNumber />
        </Form.Item>
        <Form.Item name={["execution", "mode"]} hidden>
          <Input />
        </Form.Item>

        <div className="job-form-layout">
          <div className="job-form-main">
            <section className="form-section">
              <SectionTitle step="1" title="任务信息" />
              <div className="form-grid two">
                <Form.Item name="name" label="名称" rules={[{ required: true, whitespace: true, message: "请输入名称" }]}>
                  <Input placeholder="每日同步" />
                </Form.Item>
                <Form.Item name="description" label="描述">
                  <Input placeholder="可选备注" />
                </Form.Item>
              </div>
            </section>

            <section className="form-section">
              <SectionTitle step="2" title="运行时间" />
              <ScheduleComposer
                preset={schedulePreset}
                schedule={previewSchedule}
                onPresetChange={changeSchedulePreset}
                onCalendarChange={updateCalendar}
                onIntervalChange={(seconds) => updateSchedule({ ...getSchedule(), mode: "interval", interval: { seconds } })}
                onTimeChange={updateTime}
                onYearlyDateChange={updateYearlyDate}
              />
            </section>

            <section className="form-section">
              <SectionTitle step="3" title="脚本内容" />
              <ScriptComposer
                mode={currentExecutionMode}
                currentScript={inlineScript ?? previewExecution.inlineScript}
                workingDirectory={workingDirectory}
                extensions={extensions}
                onModeChange={changeExecutionMode}
                onApplyInlineScript={applyInlineScript}
              />
            </section>

            <AdvancedSettings form={form} mode={currentExecutionMode} />
          </div>

        </div>
      </Form>
    </Modal>
  );
}

function ScheduleComposer({
  preset,
  schedule,
  onPresetChange,
  onCalendarChange,
  onIntervalChange,
  onTimeChange,
  onYearlyDateChange,
}: {
  preset: SchedulePreset;
  schedule: LaunchdSchedule;
  onPresetChange: (preset: SchedulePreset) => void;
  onCalendarChange: (patch: Partial<LaunchdSchedule["calendar"]>) => void;
  onIntervalChange: (seconds: number) => void;
  onTimeChange: (value: string) => void;
  onYearlyDateChange: (value: string) => void;
}) {
  return (
    <div className="composer-card">
      <Segmented
        block
        value={preset}
        onChange={(value) => onPresetChange(value as SchedulePreset)}
        options={[
          { label: "每天", value: "daily" },
          { label: "每月某日", value: "monthly" },
          { label: "每年某天", value: "yearly" },
          { label: "循环间隔", value: "interval" },
        ]}
      />

      {preset === "interval" ? (
        <div className="schedule-picker-stack">
          <Form.Item
            name={["schedule", "interval", "seconds"]}
            label="间隔"
            rules={[{ required: true, type: "number", min: 1, message: "间隔至少 1 秒" }]}
          >
            <InputNumber min={1} precision={0} addonAfter="秒" className="full-width" />
          </Form.Item>
          <QuickButtonRow
            options={[
              { label: "1 分钟", value: 60 },
              { label: "5 分钟", value: 300 },
              { label: "15 分钟", value: 900 },
              { label: "1 小时", value: 3600 },
            ]}
            onSelect={onIntervalChange}
          />
        </div>
      ) : (
        <div className="schedule-picker-stack">
          <div className="schedule-picker-grid">
            {preset === "monthly" && (
              <Form.Item label="每月第几天">
                <InputNumber
                  min={1}
                  max={31}
                  precision={0}
                  value={schedule.calendar.day}
                  addonAfter="日"
                  className="full-width"
                  onChange={(value) => onCalendarChange({ month: undefined, day: Number(value ?? 1) })}
                />
              </Form.Item>
            )}

            {preset === "yearly" && (
              <Form.Item label="哪一天">
                <Input type="date" value={yearlyDateInputValue(schedule)} onChange={(event) => onYearlyDateChange(event.target.value)} />
              </Form.Item>
            )}

            <Form.Item label="几点运行">
              <TimePicker
                value={dayjs(timeInputValue(schedule), "HH:mm:ss")}
                format="HH:mm:ss"
                className="full-width"
                onChange={(_, timeString) => onTimeChange(typeof timeString === "string" ? timeString : "00:00:00")}
              />
            </Form.Item>
          </div>
          <QuickButtonRow
            options={[
              { label: "09:00", value: "09:00:00" },
              { label: "12:00", value: "12:00:00" },
              { label: "18:00", value: "18:00:00" },
              { label: "23:30", value: "23:30:00" },
            ]}
            onSelect={onTimeChange}
          />
        </div>
      )}
    </div>
  );
}

function ScriptComposer({
  mode,
  currentScript,
  workingDirectory,
  extensions,
  onModeChange,
  onApplyInlineScript,
}: {
  mode: ExecutionMode;
  currentScript: string;
  workingDirectory?: string;
  extensions: Extension[];
  onModeChange: (mode: ExecutionMode) => void;
  onApplyInlineScript: (script: string) => void;
}) {
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
      const result = await generateNodeScript({ prompt: aiPrompt, currentScript });
      onApplyInlineScript(result.script);
      message.success("脚本写好了");
    } catch (err) {
      message.error(friendlyError(err));
    } finally {
      setGenerating(false);
    }
  }

  async function handleDebugScript() {
    if (!currentScript.trim()) {
      message.warning("没有可运行的脚本内容");
      return;
    }
    setDebugging(true);
    try {
      const result = await runNodeScriptDebug({ script: currentScript, workingDirectory });
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

  return (
    <div className="composer-card">
      <Segmented
        block
        value={mode === "interpreter" ? "script_path" : mode}
        onChange={(value) => onModeChange(value as ExecutionMode)}
        options={[
          { label: "写 Node.js", value: "inline_shell" },
          { label: "运行 .js 文件", value: "script_path" },
        ]}
      />

      {mode === "inline_shell" ? (
        <div className="script-editor-block">
          <Form.Item name={["execution", "inlineScript"]} rules={[{ required: true, whitespace: true, message: "请输入脚本内容" }]}>
            <CodeEditor extensions={extensions} />
          </Form.Item>

          <div className="ai-helper">
            <Input.TextArea
              value={aiPrompt}
              autoSize={{ minRows: 2, maxRows: 4 }}
              placeholder="描述你想让 Node.js 脚本做什么。比如：发一条 macOS 通知提醒我喝水，并把时间写进日志。"
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
            <Typography.Text type="secondary">直接运行当前 Node.js 内容，不会保存任务。</Typography.Text>
          </div>
          <ScriptDebugPanel result={debugResult} />
        </div>
      ) : (
        <div className="script-file-block">
          <Form.Item
            name={["execution", "scriptPath"]}
            label="Node.js 文件"
            rules={[{ required: true, whitespace: true, message: "请输入脚本路径" }]}
          >
            <Input placeholder="/Users/gavin/scripts/job.js" />
          </Form.Item>
        </div>
      )}
    </div>
  );
}

function AdvancedSettings({ form, mode }: { form: FormInstance<LaunchdJobInput>; mode: ExecutionMode }) {
  return (
    <Collapse
      ghost
      className="advanced-settings"
      items={[
        {
          key: "advanced",
          label: "高级设置",
          children: (
            <div className="advanced-settings-body">
              <Form.Item
                name={["execution", "interpreter"]}
                label="Node 命令"
                rules={mode === "interpreter" ? [{ required: true, whitespace: true, message: "请输入解释器路径" }] : undefined}
              >
                <Input placeholder="/usr/bin/env node" />
              </Form.Item>

              {mode !== "inline_shell" && (
                <Form.Item name={["execution", "arguments"]} label="参数">
                  <Input placeholder='--env prod "quoted value"' />
                </Form.Item>
              )}

              <Form.Item name={["execution", "workingDirectory"]} label="工作目录">
                <Input placeholder="/Users/gavin/project" />
              </Form.Item>

              <div className="advanced-subsection">
                <Typography.Text type="secondary">环境变量</Typography.Text>
                <Form.List name={["execution", "environment"]}>
                  {(fields, { add, remove }) => (
                    <Space orientation="vertical" className="full-width" size={8}>
                      {fields.map((field) => (
                        <div className="env-row" key={field.key}>
                          <Form.Item name={[field.name, "key"]} noStyle>
                            <Input placeholder="变量名" />
                          </Form.Item>
                          <Form.Item name={[field.name, "value"]} noStyle>
                            <Input placeholder="变量值" />
                          </Form.Item>
                          <Button icon={<DeleteOutlined />} onClick={() => remove(field.name)} />
                        </div>
                      ))}
                      <Button icon={<PlusOutlined />} onClick={() => add({ key: "", value: "" })}>
                        添加变量
                      </Button>
                    </Space>
                  )}
                </Form.List>
              </div>

              {mode === "interpreter" && (
                <Button onClick={() => form.setFieldValue(["execution", "mode"], "script_path")}>改用脚本文件模式</Button>
              )}
            </div>
          ),
        },
      ]}
    />
  );
}

function QuickButtonRow<T extends string | number>({ options, onSelect }: { options: Array<{ label: string; value: T }>; onSelect: (value: T) => void }) {
  return (
    <div className="quick-buttons">
      {options.map((option) => (
        <Button key={option.label} onClick={() => onSelect(option.value)}>
          {option.label}
        </Button>
      ))}
    </div>
  );
}

function SectionTitle({ step, title }: { step: string; title: string }) {
  return (
    <div className="section-title">
      <span>{step}</span>
      <Typography.Title level={5}>{title}</Typography.Title>
    </div>
  );
}

function CodeEditor({
  value,
  onChange,
  extensions,
}: {
  value?: string;
  onChange?: (value: string) => void;
  extensions: Extension[];
}) {
  return (
    <CodeMirror
      value={value}
      height="320px"
      width="100%"
      extensions={extensions}
      basicSetup={{ lineNumbers: true, foldGutter: true }}
      onChange={(next) => onChange?.(next)}
    />
  );
}

function detectSchedulePreset(schedule: LaunchdSchedule): SchedulePreset {
  if (schedule.mode === "interval") return "interval";
  if (schedule.calendar.month && schedule.calendar.day) return "yearly";
  if (schedule.calendar.day) return "monthly";
  return "daily";
}

function timeInputValue(schedule: LaunchdSchedule) {
  const { hour = 0, minute = 0, second = 0 } = schedule.calendar;
  return `${pad(hour)}:${pad(minute)}:${pad(second)}`;
}

function yearlyDateInputValue(schedule: LaunchdSchedule) {
  const now = new Date();
  const month = schedule.calendar.month ?? now.getMonth() + 1;
  const day = schedule.calendar.day ?? now.getDate();
  return `${now.getFullYear()}-${pad(month)}-${pad(day)}`;
}

function pad(value: number) {
  return String(value).padStart(2, "0");
}
