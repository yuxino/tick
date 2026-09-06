from pathlib import Path
p=Path(__file__).with_name('inspect-native-ui.py');s=p.read_text()
changes={
 'win32clipboard.SetClipboardText(text)':'win32clipboard.SetClipboardText(text,13)',
 "control('高级设置','Button')":"control('.*高级设置','Button')",
 "control('立即运行','Button')":"control('.*立即运行','Button')",
 "taskname=job.get('label','')":"taskname=job.get('definitionPath','')",
 "if taskname.startswith('Tick.') or taskname.startswith('com.'):":"if taskname == '\\\\Tick.'+job['id']:",
}
for old,new in changes.items():
 if s.count(old)!=1:raise SystemExit('Capture source changed; review correction')
 s=s.replace(old,new)
compile(s,str(p),'exec');p.write_text(s)
