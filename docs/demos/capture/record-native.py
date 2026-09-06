"""Record an actual installed Tick app using Windows UI Automation. Disposable runner only."""
import base64,ctypes,json,os,pathlib,re,subprocess,threading,time
import imageio_ffmpeg,win32clipboard
from pywinauto import Desktop,mouse,keyboard
from PIL import ImageGrab
OUT=pathlib.Path('trigger-review');OUT.mkdir(exist_ok=True)
ROOT=pathlib.Path(os.environ['RUNNER_TEMP'])/'tick-original-demo';ROOT.mkdir(exist_ok=True)
NAME='喝水提醒 · Demo';report={'native':True,'success':False,'source_commit':'2e0c45e4dce4c259826fa170716f6d425559ed62','scenes':[]}
stop=threading.Event();started=time.monotonic();thread=None;job=None
w=Desktop(backend='uia').window(title='Tick');w.wait('visible',timeout=25);w.set_focus();time.sleep(2)
def snap(name):ImageGrab.grab().save(OUT/(name+'.png'))
def dump(name):
 data=[]
 for c in w.descendants():
  try:
   r=c.rectangle();data.append({'type':c.element_info.control_type,'name':c.window_text(),'id':c.element_info.automation_id,'rect':[r.left,r.top,r.right,r.bottom],'enabled':c.is_enabled()})
  except Exception:pass
 (OUT/(name+'.json')).write_text(json.dumps(data,ensure_ascii=False,indent=2),encoding='utf-8');snap(name)
def mark(caption):
 report['scenes'].append({'caption':caption,'time':time.monotonic()-started});snap('scene-'+str(len(report['scenes'])));time.sleep(1.4)
def record():
 im=ImageGrab.grab();width,height=im.size;width-=width%2;height-=height%2;report['dimensions']=[width,height]
 writer=imageio_ffmpeg.write_frames(str(OUT/'raw.mp4'),(width,height),fps=10,codec='libx264',pix_fmt_in='rgb24',pix_fmt_out='yuv420p',quality=8,macro_block_size=1)
 writer.send(None);t=time.monotonic();n=0
 try:
  while not stop.is_set():
   writer.send(ImageGrab.grab().convert('RGB').crop((0,0,width,height)).tobytes());n+=1;time.sleep(max(0,t+n/10-time.monotonic()))
 finally:writer.close()
def paste(c,text):
 c.click_input();keyboard.send_keys('^a');win32clipboard.OpenClipboard();win32clipboard.EmptyClipboard();win32clipboard.SetClipboardText(text,13);win32clipboard.CloseClipboard();keyboard.send_keys('^v');time.sleep(.5)
def control(title,ctype=None):
 opts={'title_re':title}
 if ctype:opts['control_type']=ctype
 return w.child_window(**opts)
def ensure_visible(c):
 for _ in range(8):
  r=c.rectangle()
  if 142<=r.top and r.bottom<=680:return c
  mouse.scroll(coords=(910,550),wheel_dist=-3 if r.bottom>680 else 3);time.sleep(.25)
 return c
def read_job():
 paths=list((pathlib.Path(os.environ['APPDATA'])/'tick').glob('**/scheduled-jobs.json'))
 for p in paths:
  for j in json.loads(p.read_text(encoding='utf-8')):
   if j['name']==NAME:return j
 return None
try:
 ps="$w=New-Object -ComObject WScript.Shell; $null=$w.Popup('Your task ran successfully. Time for a short break!',20,'Tick demo completed',64)"
 enc=base64.b64encode(ps.encode('utf-16-le')).decode()
 js="const {spawnSync}=require('node:child_process');\nconsole.log('Tick demo: task started');\nspawnSync('powershell.exe',['-NoProfile','-WindowStyle','Hidden','-EncodedCommand',"+json.dumps(enc)+"],{windowsHide:true});\nconsole.log('Tick demo: popup dismissed; task completed');\n"
 script=ROOT/'reminder.js';script.write_text(js,encoding='utf-8')
 thread=threading.Thread(target=record);thread.start();started=time.monotonic()
 control('手动填写','Button').click_input();time.sleep(1)
 dump('form')
 paste(w.child_window(auto_id='name',control_type='Edit'),NAME)
 paste(w.child_window(auto_id='description',control_type='Edit'),'真实执行一次，弹出提醒，再查看日志。')
 control('运行 \\.js 文件','Text').click_input();time.sleep(.6)
 path=w.child_window(auto_id='execution_scriptPath',control_type='Edit');ensure_visible(path);paste(path,str(script))
 advanced=control('.*高级设置','Button')
 if not advanced.exists():advanced=control('高级设置','Text')
 ensure_visible(advanced);advanced.click_input();time.sleep(1.2);mouse.scroll(coords=(910,550),wheel_dist=-4);time.sleep(.6)
 interpreter=w.child_window(auto_id='execution_interpreter',control_type='Edit',visible_only=False);ensure_visible(interpreter);paste(interpreter,os.environ['DEMO_NODE'])
 mark('配置一个真实的提醒脚本')
 save=control('^保\\s*存$','Button');save.invoke()
 for _ in range(30):
  time.sleep(.5);job=read_job()
  if job:break
 if not job:raise RuntimeError('UI save did not produce a real task registry entry')
 report['task_id']=job['id'];report['task_saved']=True
 time.sleep(1.7);mark('任务已保存，准备立即运行')
 control('^play-circle 立即运行$','Button').click_input();report['trigger_clicked']=True
 popup=None
 for _ in range(80):
  h=ctypes.windll.user32.FindWindowW(None,'Tick demo completed')
  if h:popup=h;break
  time.sleep(.4)
 if not popup:raise RuntimeError('No actual task popup was observed')
 popup_window=Desktop(backend='uia').window(handle=popup)
 popup_window.wait('visible',timeout=8)
 popup_window.set_focus();time.sleep(1)
 report['popup_visible']=popup_window.is_visible()
 report['popup_foreground']=ctypes.windll.user32.GetForegroundWindow()==popup
 if not (report['popup_visible'] and report['popup_foreground']):raise RuntimeError('Real task popup was not visibly foregrounded')
 report['popup_observed']=True;mark('系统任务已触发，原生提醒弹窗出现');snap('poster')
 time.sleep(2);ctypes.windll.user32.PostMessageW(popup,0x0010,0,0)
 log_path=pathlib.Path(job['stdoutPath'])
 log=''
 for _ in range(60):
  if log_path.exists():log=log_path.read_text(encoding='utf-8',errors='replace')
  if 'task completed' in log:break
  time.sleep(.5)
 if 'task completed' not in log:raise RuntimeError('Real completion stdout missing')
 report['completion_stdout_verified']=True;report['log']=log
 control('实时日志','TabItem').click_input();time.sleep(2.2);mark('完成后，在实时日志里确认结果');snap('log')
 report['success']=True
except Exception as e:
 report['error']=type(e).__name__+': '+str(e)[:1200];dump('failure')
finally:
 stop.set()
 if thread:thread.join(timeout=15)
 if job:
  # Delete only this demo's owned task, never other user/system tasks.
  taskname=job.get('definitionPath','')
  if taskname == '\\Tick.'+job['id']:
   r=subprocess.run(['schtasks','/Delete','/TN',taskname,'/F'],capture_output=True,text=True);report['task_cleanup_returncode']=r.returncode
 win32clipboard.OpenClipboard();win32clipboard.EmptyClipboard();win32clipboard.CloseClipboard()
 (OUT/'report.json').write_text(json.dumps(report,ensure_ascii=False,indent=2),encoding='utf-8')
if not report['success']:raise SystemExit('Native trigger capture failed; review UI diagnostics')
