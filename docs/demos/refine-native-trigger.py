from pathlib import Path
p=Path(__file__).with_name('inspect-native-ui.py');s=p.read_text(encoding='utf-8')
changes={
 'win32clipboard.SetClipboardText(text)':'win32clipboard.SetClipboardText(text,13)',
 "control('高级设置','Button')":"control('.*高级设置','Button')",
 "control('立即运行','Button')":"control('^play-circle 立即运行$','Button')",
 "taskname=job.get('label','')":"taskname=job.get('definitionPath','')",
 "if taskname.startswith('Tick.') or taskname.startswith('com.'):":"if taskname == '\\\\Tick.'+job['id']:",
 "w.child_window(auto_id='execution_interpreter',control_type='Edit')":"w.child_window(auto_id='execution_interpreter',control_type='Edit',visible_only=False)",
 "ensure_visible(advanced);advanced.click_input();time.sleep(.5)":"ensure_visible(advanced);advanced.click_input();time.sleep(1.2);mouse.scroll(coords=(910,550),wheel_dist=-4);time.sleep(.6)",
 "time.sleep(1);control(re.escape(NAME),'Text').click_input();time.sleep(.7);mark('任务已保存，准备立即运行')":"time.sleep(1.7);mark('任务已保存，准备立即运行')",
}
for old,new in changes.items():
 if s.count(old)!=1:raise SystemExit('Capture source changed; review correction')
 s=s.replace(old,new)
compile(s,str(p),'exec');p.write_text(s,encoding='utf-8')
