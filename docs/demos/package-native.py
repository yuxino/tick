"""Package a visually reviewed native Tick task run; never changes application code."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
from PIL import Image, ImageDraw, ImageFont

ROOT = Path.cwd()
SOURCE = Path(os.environ.get('NATIVE_REVIEW', 'native-review'))
OLD = Path(os.environ.get('OLD_DEMO', 'docs/demos'))
OUT = ROOT / 'native-deliverables' / 'docs/demos'
WORK = ROOT / 'native-package-work'
OUT.mkdir(parents=True, exist_ok=True)
WORK.mkdir(parents=True, exist_ok=True)
EXPECTED_PREVIOUS = 'd717070547118f1abe65a3903dfbfd7b05ac749a9dfb1a575db7f36be63cb337'
FONT = '/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc'

def sha(p):
    return hashlib.sha256(p.read_bytes()).hexdigest()

def duration(p):
    return float(subprocess.check_output(['ffprobe','-v','error','-show_entries','format=duration','-of','csv=p=0',str(p)],text=True))

def ff(args):
    subprocess.run(['ffmpeg','-hide_banner','-loglevel','error','-y',*args], check=True)

report = json.loads((SOURCE/'report.json').read_text(encoding='utf-8'))
for flag in ['native','success','task_saved','trigger_clicked','popup_visible','popup_foreground','popup_observed','completion_stdout_verified']:
    if report.get(flag) is not True:
        raise SystemExit('Native capture did not pass: ' + flag)
if report.get('task_cleanup_returncode') != 0:
    raise SystemExit('Demo task cleanup did not pass')
if report['dimensions'] != [1024,768] or len(report['scenes']) != 4:
    raise SystemExit('Recording dimensions or scene layout changed; review before publishing')
previous = json.loads((OLD/'provenance.json').read_text(encoding='utf-8'))
if sha(OLD/'demo.mp4') != EXPECTED_PREVIOUS or previous['sha256'] != EXPECTED_PREVIOUS:
    raise SystemExit('Existing approved demo changed; refusing to overwrite')

english = [
    'Configure an original reminder script in the installed app.',
    'Save the task, then click Run now.',
    'The example script opens a real Windows reminder dialog.',
    'Dismiss the dialog and verify the real completion log.',
]
def card(index):
    im = Image.new('RGB', (1280,900), '#f5f4f8')
    d = ImageDraw.Draw(im)
    d.text((32,7),'Tick',font=ImageFont.truetype(FONT,28),fill='#342f40')
    d.text((525,18),'WINDOWS NATIVE RUN / 原生运行实录 · 10× 操作',font=ImageFont.truetype(FONT,14),fill='#76638b')
    d.text((34,835),report['scenes'][index]['caption'],font=ImageFont.truetype(FONT,20),fill='#342f40')
    d.text((35,869),english[index],font=ImageFont.truetype(FONT,13),fill='#81758d')
    return im

raw = SOURCE/'raw.mp4'
raw_duration = duration(raw)
ends = [min(x['time']+.45, raw_duration) for x in report['scenes'][:3]] + [raw_duration]
start = 0.0
holds = [.8,.8,2.0,2.0]
parts = []
segments = []
for i,end in enumerate(ends):
    if end <= start:
        raise SystemExit('Invalid scene timing')
    background=WORK/f'card-{i}.png';card(i).save(background)
    part=WORK/f'segment-{i}.mp4'
    filters=f'[0:v]trim=start={start:.6f}:end={end:.6f},setpts=(PTS-STARTPTS)/10,fps=30,tpad=stop_mode=clone:stop_duration={holds[i]},setsar=1[src];[1:v][src]overlay=128:52:shortest=1,format=yuv420p[v]'
    ff(['-i',str(raw),'-loop','1','-framerate','30','-i',str(background),'-filter_complex',filters,'-map','[v]','-an','-c:v','libx264','-preset','fast','-crf','21','-pix_fmt','yuv420p','-threads','2','-movflags','+faststart',str(part)])
    parts.append(part)
    segments.append({'kind':'native_task_run','caption':report['scenes'][i]['caption'],'raw_start':start,'raw_end':end,'action_speed':10,'result_hold':holds[i],'seconds':duration(part)})
    start=end
concat=WORK/'segments.txt'
concat.write_text('\n'.join("file '"+str(p.resolve()).replace("'","'\\''")+"'" for p in parts)+'\n')
ff(['-f','concat','-safe','0','-i',str(concat),'-c','copy','-movflags','+faststart',str(WORK/'native.mp4')])
ff(['-i',str(OLD/'demo.mp4'),'-i',str(WORK/'native.mp4'),'-filter_complex','[0:v]fps=30,setsar=1,setpts=PTS-STARTPTS[a];[1:v]fps=30,setsar=1,setpts=PTS-STARTPTS[b];[a][b]concat=n=2:v=1:a=0[v]','-map','[v]','-an','-c:v','libx264','-preset','fast','-crf','21','-pix_fmt','yuv420p','-threads','2','-movflags','+faststart',str(OUT/'demo.mp4')])
poster=card(2);poster.paste(Image.open(SOURCE/'poster.png').convert('RGB'),(128,52));poster.save(OUT/'poster.png')
ff(['-i',str(OUT/'demo.mp4'),'-filter_complex','fps=8,scale=800:-1:flags=lanczos,split[a][b];[a]palettegen=max_colors=112[p];[b][p]paletteuse=dither=bayer:bayer_scale=5:diff_mode=rectangle','-loop','0',str(OUT/'preview.gif')])
for name in ['demo.mp4','preview.gif']:
    ff(['-i',str(OUT/name),'-f','null','-'])
if abs(duration(OUT/'demo.mp4')-duration(OUT/'preview.gif'))>.3:
    raise SystemExit('Animation and video timing differ')
if not 20 < duration(OUT/'demo.mp4') < 40:
    raise SystemExit('Unexpected final duration')
for p in OUT.iterdir():
    if p.is_file() and p.stat().st_size>8_000_000:
        raise SystemExit('Media exceeded size budget')

verification={k:report[k] for k in ['native','success','source_commit','scenes','dimensions','task_saved','trigger_clicked','popup_visible','popup_foreground','popup_observed','completion_stdout_verified','log','task_cleanup_returncode']}
verification.update({'capture_run':34007989748,'capture_artifact':9981574158,'environment':'Disposable GitHub-hosted Windows Server 2025 desktop; installed unmodified Tick x64 build','trigger':'Manual Run now, routed through the actual Windows task scheduler','dialog':'Created by the original example script. The recorder focused that existing real dialog before taking the poster; no fake dialog was drawn.'})
(OUT/'native-verification.json').write_text(json.dumps(verification,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
provenance={'project':'tick','source_commit':previous['source_commit'],'duration':duration(OUT/'demo.mp4'),'sha256':sha(OUT/'demo.mp4'),'previous_media_sha256':previous['sha256'],'native_source_commit':report['source_commit'],'native_recording_sha256':sha(raw),'capture_run':34007989748,'capture_artifact':9981574158,'native_execution_verified':True,'segments':[{'kind':'existing_configuration_tour','native':False,'seconds':duration(OLD/'demo.mp4'),'action_speed':10,'result_hold':.8},*segments],'disclosure':'The first segment retains the original frontend-only configuration tour. The closing segment is an actual installed Windows app task save, manual Run now, real reminder dialog and real completion stdout. Popup comes from the sample script, not a built-in reminder feature. Actions are edited to 10x with result holds; not a scheduler-latency benchmark or proof of time-based activation on every supported OS.'}
provenance['media']={p.name:{'bytes':p.stat().st_size,'sha256':sha(p)} for p in OUT.iterdir() if p.is_file()}
(OUT/'provenance.json').write_text(json.dumps(provenance,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
(OUT/'README.md').write_text('''# Tick demonstration

The video starts with the existing fast configuration tour, then shows an **actual installed Windows app** saving and manually triggering an original reminder script. A real Windows dialog appears; after dismissal, the actual completion output is visible in Tick's log tab.

The popup is produced by the example script, not a built-in reminder feature. The task is started with **Run now** through the real Windows scheduler; this recording does not claim that a calendar-based trigger was awaited.

The earlier configuration tour uses the real frontend with isolated native fixtures. The closing task-run segment uses the unmodified x64 installer on a disposable Windows Server 2025 CI desktop. No user files or unrelated tasks are accessed; the one demo task is deleted afterwards.

Actions play at **10x**, with short result holds (2 seconds for the popup and final log). The recorder focuses the already-created dialog so it is not hidden behind Tick. The video and GIF have matching timing; this is not a latency benchmark.

- `demo.mp4`: configuration and actual-run demonstration, without audio.
- `preview.gif`: inline README animation.
- `poster.png`: actual native popup, with an outer documentation caption.
- `native-verification.json`: real save, trigger, popup visibility, stdout and cleanup checks.
- `provenance.json`: source/build references, segment timing and media hashes.

No application code, dependencies, scheduler behavior, signing configuration, version or release was changed for this demo.
''',encoding='utf-8')
capture=OUT/'capture';capture.mkdir(exist_ok=True)
shutil.copyfile(SOURCE/'capture-native.py',capture/'record-native.py')
shutil.copyfile(Path(__file__),capture/'package-native.py')
print(json.dumps({'duration':provenance['duration'],'media':provenance['media']},ensure_ascii=False,indent=2))
