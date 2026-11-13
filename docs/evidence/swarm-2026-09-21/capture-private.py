"""Host-local control of the R5 probe ONLY. No daily display/bus fallback."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import time

parser = argparse.ArgumentParser()
parser.add_argument('root', type=Path)
parser.add_argument('output', type=Path)
parser.add_argument('--app', choices=['okular', 'dolphin'], required=True)
parser.add_argument('--click', nargs=2, type=int)
parser.add_argument('--key')
parser.add_argument('--prepare', action='store_true')
parser.add_argument('--demo', action='store_true')
args = parser.parse_args()
root = args.root.resolve()
env = json.loads((root / 'environment.json').read_text())
assert root.parent == Path('/tmp') and root.name.startswith('r5-')
assert env['HOME'] == str(root / 'home')
assert env['XDG_RUNTIME_DIR'] == str(root / 'runtime')
assert env['NIRI_SOCKET'].startswith(str(root / 'runtime/niri.'))
assert env['DBUS_SYSTEM_BUS_ADDRESS'] == 'unix:path=/nonexistent-r5-bus'
assert env['ATSPI_DBUS_IMPLEMENTATION'] == 'dbus-daemon'
display = json.loads((root / 'display.json').read_text())
assert display == {'display': env['R5_X_DISPLAY'], 'pid': int(env['R5_XVFB_PID'])}
assert b'/Xvfb\0-displayfd\0' in Path(f"/proc/{display['pid']}/cmdline").read_bytes()
env['DISPLAY'] = display['display']
args.output.mkdir(parents=True, exist_ok=True)


def run(*cmd):
    return subprocess.check_output(cmd, env=env, text=True, timeout=15)


xdotool = '/nix/store/824222776wmwpkrl8k3g4zscnynrlpi6-xdotool-4.20260303.1/bin/xdotool'
if args.prepare:
    xid = run(xdotool, 'search', '--name', '^niri$').split()[0]
    run(xdotool, 'windowsize', xid, '1280', '720', 'windowfocus', xid)
    time.sleep(2)
    windows = json.loads(run('niri', 'msg', '--json', 'windows'))
    dolphin = next(w for w in windows if 'dolphin' in w['app_id'])
    run('niri', 'msg', 'action', 'focus-window', '--id', str(dolphin['id']))
    run(xdotool, 'key', 'F9', 'ctrl+m')  # hide mount sidebar; expose real Dolphin menubar
    time.sleep(2)
windows = json.loads(run('niri', 'msg', '--json', 'windows'))
window = next(w for w in windows if args.app in w['app_id'].lower())
run('niri', 'msg', 'action', 'focus-window', '--id', str(window['id']))
time.sleep(2)
active = json.loads((root / 'cache/noctalia-appmenu/active.json').read_text())
assert active['source'] == 'atspi', active
assert active['focus_pid'] == window['pid'], (active, window)
assert active['menu']['children'], active
(args.output / (args.app + '-active.json')).write_text(json.dumps(active, indent=2) + '\n')
(args.output / (args.app + '-windows.json')).write_text(json.dumps(windows, indent=2) + '\n')
xdotool = '/nix/store/824222776wmwpkrl8k3g4zscnynrlpi6-xdotool-4.20260303.1/bin/xdotool'
if args.click:
    run(xdotool, 'mousemove', *map(str, args.click), 'click', '1')
if args.key:
    run(xdotool, 'key', args.key)
time.sleep(1)
subprocess.run(['/nix/store/j5xn6ib6mfy7l7h2fxfa16agli24f0h7-ffmpeg-full-9.0.1-bin/bin/ffmpeg', '-v', 'error', '-y', '-f', 'x11grab', '-video_size',
                '1280x720', '-i', env['DISPLAY'], '-frames:v', '1',
                str(args.output / (args.app + '.png'))], env=env, check=True, timeout=15)
print(args.app, 'native menu verified:', active['focus_pid'],
      [c['label'] for c in active['menu']['children']])
if args.demo:
    assert args.app == 'okular'
    ffmpeg = '/nix/store/j5xn6ib6mfy7l7h2fxfa16agli24f0h7-ffmpeg-full-9.0.1-bin/bin/ffmpeg'
    recorder = subprocess.Popen([ffmpeg, '-v', 'error', '-y', '-f', 'x11grab',
        '-video_size', '1280x720', '-framerate', '10', '-i', env['DISPLAY'],
        '-t', '16', '-an', '-c:v', 'libx264', '-preset', 'veryfast', '-threads', '8',
        '-pix_fmt', 'yuv420p', str(args.output / 'demo.mp4')], env=env)
    try:
        time.sleep(3)
        help_menu = next(c for c in active['menu']['children'] if c['label'] == 'Help')
        about = next(c for c in help_menu['children'] if c['label'] == 'About Okular')
        result = run('noctalia-appmenu-bridge', 'atspi-click', about['service'], about['path'],
                     '--winid', str(window['id']))
        time.sleep(2)
        after = json.loads(run('niri', 'msg', '--json', 'windows'))
        (args.output / 'action.json').write_text(json.dumps({'method': 'bridge atspi-click (not popup click)',
            'label': about['label'], 'stdout': result, 'windows_after': after}, indent=2) + '\n')
        assert any('About Okular' in w['title'] for w in after), after
        subprocess.run([ffmpeg, '-v', 'error', '-y', '-f', 'x11grab', '-video_size', '1280x720',
            '-i', env['DISPLAY'], '-frames:v', '1', str(args.output / 'poster.png')], env=env, check=True, timeout=10)
        time.sleep(2)
        run(xdotool, 'key', 'Escape')
        subprocess.run(['python3', __file__, str(root), str(args.output), '--app', 'dolphin'], check=True, timeout=20)
        recorder.wait(timeout=20)
        assert recorder.returncode == 0
    finally:
        if recorder.poll() is None:
            recorder.terminate()
            recorder.wait(timeout=5)

