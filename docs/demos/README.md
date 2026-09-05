# tick expanded interface demonstration

This replaces the earlier three-to-four-scene, 2x demo with **10 recorded scenes** from the actual production frontend at `cfd84896f682d80967f7791b1221f1097dd06ad8`.

**Pacing:** every source action interval is played at **10x**, followed by a **0.8-second result hold**. The final clip lasts 20.83 seconds; it is not a uniformly accelerated full video. The GIF and MP4 share the same timing. The fast-forward and sample-data labels remain visible.

**Scope:** Task configuration UI · no task saved or executed. The browser harness substitutes native API boundaries with original local examples; this is not native macOS/Windows end-to-end validation. No user credentials, personal files, live provider output or upstream comic content are included. Satori question composition is shown without submitting an AI request; no answer is fabricated.

## Scenes

1. 01 / Name the task and add a note / 创建任务，先写清要做什么
2. 02 / Daily schedule preset / 每天九点，快速选择时间
3. 03 / Monthly schedule / 每月执行，也可以直接配置
4. 04 / Yearly schedule / 一年一次的事，也不用记在脑子里
5. 05 / Recurring interval presets / 换成每十五分钟循环
6. 06 / Inline script editor / 直接写 Node.js 脚本
7. 07 / Use an existing script file / 已有脚本，也可以直接填写路径
8. 08 / Advanced execution options / 需要时，再展开参数与工作目录
9. 09 / Configure non-secret example variables / 给脚本配置自己的环境变量
10. 10 / Calendar view; no system task was created / 日程视图，集中查看安排

## Files

`preview.gif` is the inline README preview; `demo.mp4` is the complete silent H.264 video. `poster.png` is an actual recorded result frame. `provenance.json` records source, pacing, scene boundaries and media hashes.

The reproducible documentation-only recorder is `yuxino/kiri/docs/demos/capture/expanded.py`. It is not loaded by the applications. The shipped application code, versions, signing and update workflows remain unchanged.
