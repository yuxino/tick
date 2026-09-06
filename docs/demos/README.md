# Tick demonstration

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
