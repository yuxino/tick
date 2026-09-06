"""Documentation-only capture of an installed, unmodified Tick app on a disposable runner."""
import asyncio, base64, ctypes, json, os, pathlib, subprocess, threading, time
from PIL import ImageGrab
from playwright.async_api import async_playwright
import imageio_ffmpeg

OUT=pathlib.Path('trigger-review'); OUT.mkdir(exist_ok=True)
REPORT={'source_commit':'2e0c45e4dce4c259826fa170716f6d425559ed62','native':True,'success':False,'steps':[]}
STOP=threading.Event()
def desktop_record():
    im=ImageGrab.grab().convert('RGB'); w,h=im.size; w-=w%2; h-=h%2
    REPORT['dimensions']=[w,h]
    writer=imageio_ffmpeg.write_frames(str(OUT/'raw.mp4'),(w,h),fps=10,codec='libx264',pix_fmt_in='rgb24',pix_fmt_out='yuv420p',quality=8,macro_block_size=1)
    writer.send(None)
    start=time.monotonic();frame=0
    try:
        while not STOP.is_set():
            writer.send(ImageGrab.grab().convert('RGB').crop((0,0,w,h)).tobytes());frame+=1
            time.sleep(max(0,start+frame/10-time.monotonic()))
    finally: writer.close()
def snap(name): ImageGrab.grab().save(OUT/(name+'.png'))
def find_popup():
    return ctypes.windll.user32.FindWindowW(None,'Tick demo completed')
async def main():
    job=None;page=None;thread=None
    async with async_playwright() as pw:
        browser=None
        for _ in range(45):
            try: browser=await pw.chromium.connect_over_cdp('http://127.0.0.1:9222',timeout=1500);break
            except Exception: await asyncio.sleep(1)
        if browser is None:
            snap('no-cdp');raise RuntimeError('Installed WebView2 debugging endpoint unavailable')
        try:
            for ctx in browser.contexts:
                for p in ctx.pages:
                    if await p.evaluate('!!window.__TAURI_INTERNALS__'):page=p;break
            if page is None:raise RuntimeError('Native Tick page not found')
            page.set_default_timeout(10000)
            await page.wait_for_timeout(2000)
            async def invoke(cmd,args={}): return await page.evaluate('([c,a])=>window.__TAURI_INTERNALS__.invoke(c,a)',[cmd,args])
            # The script itself creates a real Windows popup and writes real stdout.
            ps="$w=New-Object -ComObject WScript.Shell; $null=$w.Popup('Your scheduled task ran successfully. Time for a short break!',20,'Tick demo completed',64)"
            enc=base64.b64encode(ps.encode('utf-16-le')).decode()
            script="const {spawnSync}=require('node:child_process');\nconsole.log('Tick demo: task started');\nspawnSync('powershell.exe',['-NoProfile','-WindowStyle','Hidden','-EncodedCommand',"+json.dumps(enc)+"],{windowsHide:true});\nconsole.log('Tick demo: popup dismissed; task completed');\n"
            inp={'name':'喝水提醒 · Demo','description':'真实触发一次提醒，完成后查看运行日志。','schedule':{'mode':'interval','calendar':{'month':None,'day':None,'hour':9,'minute':0,'second':0},'interval':{'seconds':3600}},'execution':{'mode':'inline_shell','inlineScript':script,'scriptPath':'','interpreter':os.environ['DEMO_NODE'],'arguments':'','workingDirectory':'','environment':[]}}
            job=await invoke('save_scheduled_job',{'input':inp})
            REPORT['task_id']=job['id'];REPORT['task_saved']=True
            await page.get_by_role('button',name='刷新任务',exact=True).click();await page.wait_for_timeout(1500)
            await page.get_by_text(inp['name'],exact=True).first.click();await page.wait_for_timeout(1000)
            REPORT['steps'].append('Saved through the native save_scheduled_job command and selected in the real UI')
            snap('01-task');thread=threading.Thread(target=desktop_record);thread.start();await asyncio.sleep(1)
            await page.get_by_role('button',name='立即运行',exact=True).click()
            REPORT['steps'].append('Clicked the real Run now button; Windows Task Scheduler starts Tick runner')
            for _ in range(60):
                if find_popup():break
                await asyncio.sleep(.5)
            hwnd=find_popup()
            if not hwnd:raise RuntimeError('Task did not produce its expected native popup')
            REPORT['popup_observed']=True;snap('02-popup');REPORT['popup_time']=time.time()
            await asyncio.sleep(3)
            # Dismiss the observed real popup, never an invented browser dialog.
            ctypes.windll.user32.PostMessageW(hwnd,0x0010,0,0)
            await asyncio.sleep(2)
            await page.get_by_role('tab',name='实时日志',exact=True).click();await asyncio.sleep(2)
            log=await invoke('read_scheduled_job_log',{'id':job['id'],'kind':'stdout'})
            REPORT['log']=log
            if 'task completed' not in json.dumps(log):raise RuntimeError('Native completion log missing')
            REPORT['steps'].append('Dismissed the native popup and verified real stdout completion')
            snap('03-log');await asyncio.sleep(3);REPORT['success']=True
        except Exception as e:
            REPORT['error']=str(e)[:1500];snap('failure')
            if page is not None:REPORT['ui']=await page.evaluate('document.body.innerText')
        finally:
            STOP.set()
            if thread:thread.join(timeout=10)
            if job and page:
                try:
                    await page.evaluate('(id)=>window.__TAURI_INTERNALS__.invoke("delete_scheduled_job",{id})',job['id']);REPORT['cleanup']=True
                except Exception:REPORT['cleanup']=False
            await browser.close()
    (OUT/'report.json').write_text(json.dumps(REPORT,ensure_ascii=False,indent=2),encoding='utf-8')
    if not REPORT['success']:raise SystemExit('Native trigger capture failed; inspect the sanitized report')
try:asyncio.run(main())
except Exception as e:
    REPORT['error']=str(e)[:1500];(OUT/'report.json').write_text(json.dumps(REPORT,ensure_ascii=False,indent=2),encoding='utf-8');raise
