import { DeleteOutlined, PlusOutlined } from "@ant-design/icons";
import CodeMirror from "@uiw/react-codemirror";
import { javascript } from "@codemirror/lang-javascript";
import { StreamLanguage } from "@codemirror/language";
import { shell } from "@codemirror/legacy-modes/mode/shell";
import type { Extension } from "@codemirror/state";
import { Button, Divider, Form, Input, InputNumber, Modal, Segmented, Space, Typography } from "antd";
import { useEffect, useMemo } from "react";
import type { ExecutionMode, LaunchdJobInput, ScheduleMode } from "../types/launchd";
import { emptyJobInput } from "../utils/launchd";

interface JobFormModalProps {
  open: boolean;
  initialValue?: LaunchdJobInput;
  saving: boolean;
  onCancel: () => void;
  onSubmit: (input: LaunchdJobInput) => Promise<void>;
}

export function JobFormModal({ open, initialValue, saving, onCancel, onSubmit }: JobFormModalProps) {
  const [form] = Form.useForm<LaunchdJobInput>();
  const scheduleMode = Form.useWatch(["schedule", "mode"], form) as ScheduleMode | undefined;
  const executionMode = Form.useWatch(["execution", "mode"], form) as ExecutionMode | undefined;
  const interpreter = Form.useWatch(["execution", "interpreter"], form) as string | undefined;

  const extensions = useMemo(() => {
    if (interpreter?.includes("node")) {
      return [javascript({ jsx: true, typescript: true })];
    }
    return [StreamLanguage.define(shell)];
  }, [interpreter]);

  useEffect(() => {
    if (open) {
      form.setFieldsValue(initialValue ?? emptyJobInput());
    }
  }, [form, initialValue, open]);

  async function handleOk() {
    const values = await form.validateFields();
    await onSubmit(values);
  }

  return (
    <Modal
      title={initialValue?.id ? "Edit Job" : "New Job"}
      open={open}
      width={900}
      onCancel={onCancel}
      onOk={handleOk}
      confirmLoading={saving}
      okText="Save"
      destroyOnHidden
    >
      <Form form={form} layout="vertical" initialValues={emptyJobInput()} className="job-form">
        <Form.Item name="id" hidden>
          <Input />
        </Form.Item>

        <div className="form-grid two">
          <Form.Item name="name" label="Name" rules={[{ required: true, whitespace: true, message: "Name is required" }]}>
            <Input placeholder="Nightly sync" />
          </Form.Item>
          <Form.Item name="description" label="Description">
            <Input placeholder="Optional note" />
          </Form.Item>
        </div>

        <Divider titlePlacement="start">Schedule</Divider>
        <Form.Item name={["schedule", "mode"]} label="Mode">
          <Segmented
            block
            options={[
              { label: "Calendar", value: "calendar" },
              { label: "Interval", value: "interval" },
            ]}
          />
        </Form.Item>

        {scheduleMode === "interval" ? (
          <Form.Item
            name={["schedule", "interval", "seconds"]}
            label="Every N seconds"
            rules={[{ required: true, type: "number", min: 1, message: "Interval must be at least 1 second" }]}
          >
            <InputNumber min={1} precision={0} className="full-width" />
          </Form.Item>
        ) : (
          <div className="form-grid five">
            <Form.Item name={["schedule", "calendar", "month"]} label="Month">
              <InputNumber min={1} max={12} precision={0} placeholder="*" className="full-width" />
            </Form.Item>
            <Form.Item name={["schedule", "calendar", "day"]} label="Day">
              <InputNumber min={1} max={31} precision={0} placeholder="*" className="full-width" />
            </Form.Item>
            <Form.Item name={["schedule", "calendar", "hour"]} label="Hour">
              <InputNumber min={0} max={23} precision={0} placeholder="0" className="full-width" />
            </Form.Item>
            <Form.Item name={["schedule", "calendar", "minute"]} label="Minute">
              <InputNumber min={0} max={59} precision={0} placeholder="0" className="full-width" />
            </Form.Item>
            <Form.Item
              name={["schedule", "calendar", "second"]}
              label="Second"
              rules={[{ required: true, type: "number", min: 0, max: 59, message: "Second must be 0-59" }]}
            >
              <InputNumber min={0} max={59} precision={0} className="full-width" />
            </Form.Item>
          </div>
        )}

        <Divider titlePlacement="start">Command</Divider>
        <Form.Item name={["execution", "mode"]} label="Execution">
          <Segmented
            block
            options={[
              { label: "Inline sh", value: "inline_shell" },
              { label: "Script path", value: "script_path" },
              { label: "Interpreter", value: "interpreter" },
            ]}
          />
        </Form.Item>

        <div className="form-grid two">
          <Form.Item
            name={["execution", "interpreter"]}
            label="Interpreter"
            rules={executionMode === "interpreter" ? [{ required: true, whitespace: true, message: "Interpreter is required" }] : []}
          >
            <Input placeholder="/bin/sh, /opt/homebrew/bin/node" />
          </Form.Item>
          <Form.Item
            name={["execution", "scriptPath"]}
            label="Script path"
            rules={executionMode === "script_path" ? [{ required: true, whitespace: true, message: "Script path is required" }] : []}
          >
            <Input placeholder="/Users/gavin/scripts/job.js" />
          </Form.Item>
        </div>

        <div className="form-grid two">
          <Form.Item name={["execution", "arguments"]} label="Arguments">
            <Input placeholder='--env prod "quoted value"' />
          </Form.Item>
          <Form.Item name={["execution", "workingDirectory"]} label="Working directory">
            <Input placeholder="/Users/gavin/project" />
          </Form.Item>
        </div>

        {executionMode === "inline_shell" && (
          <Form.Item
            name={["execution", "inlineScript"]}
            label="Inline script"
            rules={[{ required: true, whitespace: true, message: "Inline shell script is required" }]}
          >
            <CodeEditor extensions={extensions} />
          </Form.Item>
        )}

        <Divider titlePlacement="start">Environment</Divider>
        <Form.List name={["execution", "environment"]}>
          {(fields, { add, remove }) => (
            <Space direction="vertical" className="full-width" size={8}>
              {fields.map((field) => (
                <div className="env-row" key={field.key}>
                  <Form.Item name={[field.name, "key"]} noStyle>
                    <Input placeholder="KEY" />
                  </Form.Item>
                  <Form.Item name={[field.name, "value"]} noStyle>
                    <Input placeholder="value" />
                  </Form.Item>
                  <Button icon={<DeleteOutlined />} onClick={() => remove(field.name)} />
                </div>
              ))}
              <Button icon={<PlusOutlined />} onClick={() => add({ key: "", value: "" })}>
                Add variable
              </Button>
            </Space>
          )}
        </Form.List>

        <Typography.Text type="secondary" className="form-note">
          Use absolute paths for node, python, scripts, and project directories. LaunchAgents do not load your interactive shell profile.
        </Typography.Text>
      </Form>
    </Modal>
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
      height="260px"
      extensions={extensions}
      basicSetup={{ lineNumbers: true, foldGutter: true }}
      onChange={(next) => onChange?.(next)}
    />
  );
}
