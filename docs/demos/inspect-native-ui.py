import json,time,pathlib
from pywinauto import Desktop
from PIL import ImageGrab
out=pathlib.Path('trigger-review');out.mkdir(exist_ok=True)
w=Desktop(backend='uia').window(title='Tick');w.wait('visible',timeout=25);w.set_focus();time.sleep(2)
def dump(name):
 data=[]
 for c in w.descendants():
  try:data.append({'type':c.element_info.control_type,'name':c.window_text(),'id':c.element_info.automation_id,'rect':list(c.rectangle()),'enabled':c.is_enabled()})
  except Exception:pass
 (out/(name+'.json')).write_text(json.dumps(data,ensure_ascii=False,indent=2),encoding='utf-8');ImageGrab.grab().save(out/(name+'.png'))
dump('initial')
w.child_window(title='手动填写',control_type='Button').click_input();time.sleep(2);dump('form')
