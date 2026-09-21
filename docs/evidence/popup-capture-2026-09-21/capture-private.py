"""Measured-host integration check: real legacy topbar -> row -> app dialog.

Run only against a fresh root printed by probe-isolated.py. No daily fallback.
--miss-topbar deliberately misses Help and MUST fail the visible-row oracle.
"""
import argparse
import csv
import io
import json
import os
from pathlib import Path
import signal
import subprocess
import time

XDOTOOL = '/nix/store/824222776wmwpkrl8k3g4zscnynrlpi6-xdotool-4.20260303.1/bin/xdotool'
FFMPEG = '/nix/store/j5xn6ib6mfy7l7h2fxfa16agli24f0h7-ffmpeg-full-9.0.1-bin/bin/ffmpeg'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('root', type=Path)
    parser.add_argument('output', type=Path)
    parser.add_argument('--prepare', action='store_true')
    parser.add_argument('--miss-topbar', action='store_true')
    args = parser.parse_args()
    root = args.root.resolve()
    assert root.parent == Path('/tmp') and root.name.startswith('r5c-')
    assert root.stat().st_uid == os.getuid() and root.stat().st_mode & 0o077 == 0
    env = json.loads((root / 'environment.json').read_text())
    assert env['HOME'] == str(root / 'home')
    assert env['XDG_RUNTIME_DIR'] == str(root / 'runtime')
    assert env['NIRI_SOCKET'].startswith(str(root / 'runtime/niri.'))
    assert env['DBUS_SYSTEM_BUS_ADDRESS'] == 'unix:path=/nonexistent-r5-bus'
    assert env['ATSPI_DBUS_IMPLEMENTATION'] == 'dbus-daemon'
    display = json.loads((root / 'display.json').read_text())
    assert display['display'] == env['R5_X_DISPLAY']
    assert display['pid'] == int(env['R5_XVFB_PID'])
    proc = Path(f"/proc/{display['pid']}")
    assert b'/Xvfb\0-displayfd\0' in (proc / 'cmdline').read_bytes()
    assert (proc / 'stat').read_text().split()[21] == display['start_ticks']
    env['DISPLAY'] = display['display']  # niri overwrites DISPLAY for Xwayland
    args.output.mkdir(parents=True, exist_ok=True)
    events = []
    started = time.monotonic()

    def run(*cmd):
        return subprocess.check_output(cmd, env=env, text=True, timeout=15)

    def save(name, value):
        (args.output / name).write_text(json.dumps(value, indent=2) + '\n')

    def event(kind, **data):
        events.append({'seconds': round(time.monotonic() - started, 3),
                       'kind': kind, **data})
        save('events.json', events)

    def windows():
        return json.loads(run('niri', 'msg', '--json', 'windows'))

    def screenshot(name):
        path = args.output / (name + '.png')
        run(FFMPEG, '-v', 'error', '-y', '-f', 'x11grab', '-video_size',
            '1280x720', '-i', env['DISPLAY'], '-frames:v', '1', str(path))
        return path

    def text_lines(path, hx):
        # OCR-only enlargement; the evidence PNG/video remains unmodified.
        ox, oy = max(0, hx - 24), 32
        crop = path.with_name(path.stem + '-ocr.png')
        run(FFMPEG, '-v', 'error', '-y', '-i', str(path), '-vf',
            f'crop={min(280, 1280 - ox)}:300:{ox}:{oy},scale=iw*3:ih*3', str(crop))
        tsv = run('tesseract', str(crop), 'stdout', '--psm', '6', 'tsv')
        groups = {}
        for row in csv.DictReader(io.StringIO(tsv), delimiter='\t', quoting=csv.QUOTE_NONE):
            if row['level'] == '5' and row['text'].strip():
                key = tuple(row[k] for k in ('block_num', 'par_num', 'line_num'))
                groups.setdefault(key, []).append(row)
        lines = []
        for words in groups.values():
            left = min(int(w['left']) for w in words)
            top = min(int(w['top']) for w in words)
            right = max(int(w['left']) + int(w['width']) for w in words)
            bottom = max(int(w['top']) + int(w['height']) for w in words)
            lines.append((' '.join(w['text'] for w in words),
                          ox + left // 3, oy + top // 3,
                          ox + right // 3, oy + bottom // 3))
        return lines

    def click(x, y, role):
        event('mouse-click', role=role, x=x, y=y)
        run(XDOTOOL, 'mousemove', str(x), str(y), 'click', '1')
        time.sleep(1)

    if args.prepare:
        assert not (root / 'prepared').exists(), 'prepare only once per private session'
        xid = run(XDOTOOL, 'search', '--name', '^niri$').split()[0]
        run(XDOTOOL, 'windowsize', xid, '1280', '720', 'windowfocus', xid)
        time.sleep(2)
        dolphin = next(w for w in windows() if 'dolphin' in w['app_id'])
        run('niri', 'msg', 'action', 'focus-window', '--id', str(dolphin['id']))
        run(XDOTOOL, 'key', 'F9', 'ctrl+m')  # setup only: hide mounts, expose menubar
        time.sleep(2)
        (root / 'prepared').touch()
        event('setup', note='Dolphin Places hidden and menubar enabled before recording')
    assert (root / 'prepared').exists(), 'run --prepare on this fresh session first'
    assert not any(w['title'].startswith('About ') for w in windows()), 'fresh dialog state required'
    recorder = subprocess.Popen([FFMPEG, '-v', 'error', '-y', '-f', 'x11grab',
        '-video_size', '1280x720', '-framerate', '10', '-i', env['DISPLAY'],
        '-an', '-c:v', 'libx264', '-preset', 'veryfast', '-threads', '8',
        '-pix_fmt', 'yuv420p', str(args.output / 'demo.mp4')], env=env)
    try:
        for app in ('okular', 'dolphin'):
            before = windows()
            window = next(w for w in before if app in w['app_id'].lower())
            run('niri', 'msg', 'action', 'focus-window', '--id', str(window['id']))
            time.sleep(2)
            active = json.loads((root / 'cache/noctalia-appmenu/active.json').read_text())
            assert active['source'] == 'atspi' and active['focus_pid'] == window['pid']
            help_menu = next(c for c in active['menu']['children'] if c['label'] == 'Help')
            label = 'About ' + app.capitalize()
            assert any(c['label'] == label for c in help_menu['children'])
            save(app + '-active.json', active)
            save(app + '-before.json', before)
            # The topbar is y<32. Never click the application's own menubar.
            image = screenshot(app + '-before')
            tsv = run('tesseract', str(image), 'stdout', '--psm', '11', 'tsv')
            help_words = [r for r in csv.DictReader(io.StringIO(tsv), delimiter='\t', quoting=csv.QUOTE_NONE)
                          if r['text'] == 'Help' and int(r['top']) < 32]
            assert len(help_words) == 1, help_words
            word = help_words[0]
            hx = int(word['left']) + int(word['width']) // 2
            hy = int(word['top']) + int(word['height']) // 2
            click(1100 if args.miss_topbar else hx, hy, app + ' topbar Help')
            image = screenshot(app + '-popup')
            lines = text_lines(image, hx)
            save(app + '-popup-ocr.json', lines)
            matches = [line for line in lines if line[0] == label
                       and 32 < line[2] < 600 and hx - 30 <= line[1] <= hx + 400]
            assert len(matches) == 1, f'visible topbar popup row missing: {label}: {lines}'
            _, left, top, right, bottom = matches[0]
            event('visible-popup-row', app=app, label=label, bounds=matches[0][1:])
            time.sleep(2)
            click((left + right) // 2, (top + bottom) // 2, app + ' popup ' + label)
            after = windows()
            dialogs = [w for w in after if w['title'] == label and w['pid'] == window['pid']
                       and w['id'] not in {b['id'] for b in before}]
            assert len(dialogs) == 1, after
            save(app + '-after.json', after)
            screenshot(app + '-action')
            event('observed-action', app=app, dialog=dialogs[0])
            print(app, 'PASS: real topbar -> visible row -> new same-PID', label, flush=True)
            time.sleep(3)
            # Teardown only; neither focus nor close substitutes for the action.
            run('niri', 'msg', 'action', 'close-window', '--id', str(dialogs[0]['id']))
            time.sleep(1)
        event('result', passed=True)
    finally:
        recorder.send_signal(signal.SIGINT)
        recorder.wait(timeout=15)
        assert recorder.returncode in (0, 255), recorder.returncode
        (args.output / 'qml.log').write_text((root / 'shell.log').read_text())


if __name__ == '__main__':
    main()
