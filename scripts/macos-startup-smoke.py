import hashlib
import json
import os
from pathlib import Path
import plistlib
import shutil
import subprocess
import sys
import time

archive = Path(sys.argv[1]).resolve()
output = Path('startup-diagnostics').resolve()
output.mkdir(exist_ok=True)
staging = Path(os.environ['RUNNER_TEMP']) / 'ximuai-startup-test'
staging.mkdir(exist_ok=True)
subprocess.run(['tar', '-xzf', str(archive), '-C', str(staging)], check=True)
app = next(staging.glob('*.app'))
with (app / 'Contents/Info.plist').open('rb') as file:
    info = plistlib.load(file)
binary = app / 'Contents/MacOS' / info['CFBundleExecutable']
metadata = {'archive_sha256': hashlib.sha256(archive.read_bytes()).hexdigest(), 'bundle_id': info['CFBundleIdentifier']}
for name, command in [('system', ['sw_vers']), ('architecture', ['uname', '-m']), ('signature', ['codesign', '-dv', '--verbose=4', str(app)])]:
    result = subprocess.run(command, capture_output=True, text=True)
    (output / (name + '.txt')).write_text(result.stdout + result.stderr)
metadata['start_time'] = time.time()
with (output / 'stdout.log').open('w') as stdout, (output / 'stderr.log').open('w') as stderr:
    environment = dict(os.environ, RUST_BACKTRACE='full')
    process = subprocess.Popen([str(binary)], stdout=stdout, stderr=stderr, env=environment)
    metadata['pid'] = process.pid
    deadline = time.monotonic() + 30
    while process.poll() is None and time.monotonic() < deadline:
        time.sleep(0.25)
    alive = process.poll() is None
    metadata['survived_30_seconds'] = alive
    metadata['exit_code'] = process.poll()
    if alive:
        process.terminate()
        try:
            process.wait(timeout=10)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait(timeout=10)
    else:
        time.sleep(8)
if not alive:
    command = ['/usr/bin/lldb', '--batch', '-o', 'run', '-k', 'thread backtrace all', '-k', 'image list', '--', str(binary)]
    with (output / 'lldb.log').open('w') as debugger_log:
        try:
            debugger = subprocess.run(command, stdout=debugger_log, stderr=subprocess.STDOUT, timeout=45, env=dict(os.environ, RUST_BACKTRACE='full'))
            metadata['lldb_exit_code'] = debugger.returncode
        except subprocess.TimeoutExpired:
            metadata['lldb_timeout'] = True
    print('DEBUGGER:')
    print((output / 'lldb.log').read_text()[-24000:])
reports = Path.home() / 'Library/Logs/DiagnosticReports'
if reports.exists():
    for report in reports.glob('*'):
        if report.is_file() and report.stat().st_mtime >= metadata['start_time'] and ('ximu' in report.name.lower() or info['CFBundleExecutable'] in report.name):
            shutil.copy2(report, output / report.name)
metadata['end_time'] = time.time()
(output / 'result.json').write_text(json.dumps(metadata, indent=2))
print(json.dumps(metadata, indent=2))
print('STDERR:')
print((output / 'stderr.log').read_text()[-16000:])
raise SystemExit(0 if alive else 1)
