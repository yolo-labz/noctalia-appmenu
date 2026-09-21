"""Fresh private legacy session; touch PRINTED_ROOT/stop to stop owned children."""
import os, pathlib, subprocess, tempfile, time, sys, signal
capture = '--capture' in sys.argv
# Keep Unix socket paths below SUN_LEN; the harness TMPDIR is too long.
root=pathlib.Path(tempfile.mkdtemp(prefix='r5c-', dir='/tmp'))
print('Owned runtime:', root, flush=True)
for d in ('home','runtime','config','cache','data'): (root/d).mkdir(mode=0o700)
(root/'config/niri').mkdir()
(root/'config/niri/config.kdl').write_text('input { keyboard { xkb { layout "us"; }; }; }\n')
env={'PATH':'/run/current-system/sw/bin:/etc/profiles/per-user/notroot/bin','HOME':str(root/'home'),'XDG_RUNTIME_DIR':str(root/'runtime'),'XDG_CONFIG_HOME':str(root/'config'),'XDG_CACHE_HOME':str(root/'cache'),'XDG_DATA_HOME':str(root/'data'),'XDG_DATA_DIRS':'/run/current-system/sw/share','LIBGL_ALWAYS_SOFTWARE':'1','DBUS_SYSTEM_BUS_ADDRESS':'unix:path=/nonexistent-r5-bus'}
import glob, json
shell='/nix/store/xqjhgfb7cl2gidv3b4nnlbsvqgzam56a-noctalia-shell-2026-06-07_5311e14/share/noctalia-shell'
qs='/nix/store/crsydf85zdgki13arrpqa57kx61ix8a3-quickshell-2026-06-07_f308426/bin/qs'
bridge='/nix/store/33ngl01k5a6rbggh205jq2aadhnhp2n4-noctalia-appmenu-bridge-1.0.36/bin'
env['PATH']=bridge+':'+env['PATH']; env['QT_ACCESSIBILITY']='1'; env['QT_QPA_PLATFORM']='wayland'
# The broker launcher uses systemd activation; the nested bus has no user manager.
env['ATSPI_DBUS_IMPLEMENTATION']='dbus-daemon'
env['GSETTINGS_BACKEND']='memory'
env['PULSE_SERVER']='unix:'+str(root/'no-audio')
env['LANG']='C.UTF-8'
config=root/'config/noctalia'; (config/'plugins').mkdir(parents=True)
(config/'plugins/noctalia-appmenu').symlink_to(pathlib.Path.cwd()/'plugin', target_is_directory=True)
(config/'plugins.json').write_text(json.dumps({'version':2,'states':{'noctalia-appmenu':{'enabled':True}},'sources':[]}))
settings=json.loads(pathlib.Path(shell+'/Assets/settings-default.json').read_text())
settings['bar']['widgets']={'left':[{'id':'plugin:noctalia-appmenu'}],'center':[],'right':[]}
(config/'settings.json').write_text(json.dumps(settings))
(root/'cache/noctalia').mkdir()
(root/'cache/noctalia/shell-state.json').write_text(json.dumps({'changelogState':{'lastSeenVersion':'v4.7.8'}}))
launch=root/'launch.sh'
launch.write_text(f'#!/usr/bin/env bash\nnoctalia-appmenu-bridge >{root}/bridge.log 2>&1 &\nokular >{root}/okular.log 2>&1 &\ndolphin "$HOME" >{root}/dolphin.log 2>&1 &\n{qs} -p {shell} >{root}/shell.log 2>&1 &\npython3 -c \'import os,json; json.dump(dict(os.environ),open("{root}/environment.json","w"))\'\nwait\n')
env['LD_LIBRARY_PATH']=':'.join(sorted({str(pathlib.Path(p).parent) for pattern in ('libXcursor.so.1','libXrandr.so.2','libXi.so.6') for p in glob.glob('/nix/store/*/lib/'+pattern)}))
r,w=os.pipe()
log=open(root/'xvfb.log','w')
x=subprocess.Popen(['/nix/store/2ngsl5s8gmkbvav54pg2khb5ykxc4d10-xvfb-21.1.24/bin/Xvfb','-displayfd',str(w),'-screen','0','1280x720x24','-nolisten','tcp'],env=env,pass_fds=(w,),stdout=log,stderr=log)
os.close(w)
p = None
try:
 import select
 if not select.select([r],[],[],10)[0]: raise RuntimeError('Xvfb did not allocate display')
 display=os.read(r,40).decode().strip(); env['DISPLAY']=':'+display
 env['R5_X_DISPLAY']=env['DISPLAY']; env['R5_XVFB_PID']=str(x.pid)
 (root/'display.json').write_text(json.dumps({'display':env['DISPLAY'],'pid':x.pid,
  'start_ticks': pathlib.Path(f'/proc/{x.pid}/stat').read_text().split()[21]}))
 with open(root/'niri.log','w') as out:
  p=subprocess.Popen(['dbus-run-session','--','niri','-c',str(root/'config/niri/config.kdl'),'--','bash',str(launch)],env=env,stdout=out,stderr=out,start_new_session=True)
  (root/'session.json').write_text(json.dumps({'pid':p.pid, 'pgid':p.pid}))
  deadline = time.monotonic() + (480 if capture else 25)
  while p.poll() is None and time.monotonic() < deadline and not (root/'stop').exists():
   time.sleep(0.2)
  if p.poll() is None:
   os.killpg(p.pid,signal.SIGTERM); p.wait(timeout=5)
 print('Private X display: allocated; no daily display inherited')
 print('niri exit:',p.returncode)
 print((root/'niri.log').read_text())
finally:
 if p is not None and p.poll() is None:
  os.killpg(p.pid,signal.SIGTERM); p.wait(timeout=5)
 if x.poll() is None:
  x.terminate(); x.wait(timeout=5)
 os.close(r); log.close()
for name in ('bridge.log','okular.log','dolphin.log','shell.log'):
 if (root/name).exists(): print(name, (root/name).read_text()[-8000:])
for log in (root/'runtime/quickshell/by-id').glob('*/log.log'):
 print('QML log:',log.read_text()[-18000:])
print('Scratch logs:',root)
