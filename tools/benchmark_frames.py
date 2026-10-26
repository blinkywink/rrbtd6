"""Repeatable local renderer A/B benchmark; assets and results stay local.

python tools/benchmark_frames.py --exe BTD6.exe --output local_extracted/frame-bench
"""
import argparse
import csv
import json
import hashlib
import os
from pathlib import Path
import statistics
import subprocess
import sys
import math
import struct
import time
from functools import cache
import tempfile


@cache
def compiler_api():
    import ctypes
    from ctypes import wintypes
    class Entry(ctypes.Structure):
        _fields_=[('dwSize',wintypes.DWORD),('cntUsage',wintypes.DWORD),
                  ('pid',wintypes.DWORD),('heap',ctypes.c_size_t),
                  ('module',wintypes.DWORD),('threads',wintypes.DWORD),
                  ('parent',wintypes.DWORD),('priority',wintypes.LONG),
                  ('flags',wintypes.DWORD),('name',wintypes.WCHAR*260)]
    kernel=ctypes.WinDLL('kernel32',use_last_error=True)
    snapshot=kernel.CreateToolhelp32Snapshot
    snapshot.argtypes=[wintypes.DWORD,wintypes.DWORD];snapshot.restype=wintypes.HANDLE
    first=kernel.Process32FirstW;next_entry=kernel.Process32NextW
    for fn in (first,next_entry):fn.argtypes=[wintypes.HANDLE,ctypes.POINTER(Entry)];fn.restype=wintypes.BOOL
    close=kernel.CloseHandle;close.argtypes=[wintypes.HANDLE];close.restype=wintypes.BOOL
    return Entry,snapshot,first,next_entry,close


def active_compilers():
    """Read process names only; no command lines, shell calls or extra package."""
    if os.name!='nt':return []
    import ctypes
    Entry,snapshot,first,next_entry,close=compiler_api()
    handle=snapshot(2,0)
    if handle==ctypes.c_void_p(-1).value:raise ctypes.WinError(ctypes.get_last_error())
    found=[]
    try:
        entry=Entry();entry.dwSize=ctypes.sizeof(entry)
        ok=first(handle,ctypes.byref(entry))
        if not ok:raise ctypes.WinError(ctypes.get_last_error())
        while ok:
            if entry.name.casefold() in ('rustc.exe','link.exe','cl.exe','clang.exe','clang-cl.exe','lld-link.exe'):
                found.append(f'{entry.name}:{entry.pid}')
            ok=next_entry(handle,ctypes.byref(entry))
        if ctypes.get_last_error()!=18:raise ctypes.WinError(ctypes.get_last_error())
    finally:close(handle)
    return found


def wait_for_compilers():
    """A short quiet period avoids starting on the tail of a compilation."""
    if os.name!='nt':return
    began=time.perf_counter();quiet=None;reported=False
    while True:
        now=time.perf_counter();busy=active_compilers()
        if busy:
            if not reported:print('Waiting for compilers:',', '.join(busy),flush=True);reported=True
            quiet=None
        elif quiet is None:quiet=now
        elif now-quiet>=2:return
        if now-began>300:raise RuntimeError('No compiler-free benchmark window within five minutes')
        time.sleep(.5)


def freeze_inputs(source,output):
    """Only simulation JSON and preferences are copied; assets remain in place."""
    snapshot=Path(tempfile.mkdtemp(prefix='simulation-inputs-',dir=output))
    for relative in ('game/game.json','maps/tutorial/map.json','settings.json'):
        path=source/relative
        for attempt in range(5):
            try:
                data=path.read_bytes() if path.exists() else (b'{}' if relative=='settings.json' else None)
                if data is None and relative.startswith('game/'):raise FileNotFoundError(path)
                if data is not None:json.loads(data)
                break
            except (ValueError,OSError):
                if attempt==4:raise
                time.sleep(.2)
        if data is None:
            if relative.startswith('game/'):raise FileNotFoundError(path)
            continue
        target=snapshot/relative;target.parent.mkdir(parents=True,exist_ok=True);target.write_bytes(data)
    return snapshot


def run_measured(command,**kwargs):
    """CPU is whole-process time (including startup), not gameplay utilization."""
    sample=lambda:None
    cpu=lambda:None
    peak_private=peak_resident=None
    began=time.perf_counter();compiler_overlap=set();next_compiler_poll=began
    with subprocess.Popen(command,**kwargs) as process:
        if os.name=='nt':
            import ctypes
            from ctypes import wintypes
            class Counters(ctypes.Structure):
                _fields_=[('cb',wintypes.DWORD),('PageFaultCount',wintypes.DWORD)]+[(k,ctypes.c_size_t) for k in ('PeakWorkingSetSize','WorkingSetSize','QuotaPeakPagedPoolUsage','QuotaPagedPoolUsage','QuotaPeakNonPagedPoolUsage','QuotaNonPagedPoolUsage','PagefileUsage','PeakPagefileUsage','PrivateUsage')]
            get_mem=ctypes.WinDLL('psapi').GetProcessMemoryInfo
            get_mem.argtypes=[wintypes.HANDLE,ctypes.POINTER(Counters),wintypes.DWORD];get_mem.restype=wintypes.BOOL
            get_times=ctypes.WinDLL('kernel32').GetProcessTimes
            get_times.argtypes=[wintypes.HANDLE]+[ctypes.POINTER(wintypes.FILETIME)]*4;get_times.restype=wintypes.BOOL
            def sample():
                nonlocal peak_private,peak_resident
                c=Counters();c.cb=ctypes.sizeof(c)
                if get_mem(int(process._handle),ctypes.byref(c),c.cb):
                    peak_private=max(peak_private or 0,c.PeakPagefileUsage/1048576)
                    peak_resident=max(peak_resident or 0,c.PeakWorkingSetSize/1048576)
            def cpu():
                values=[wintypes.FILETIME() for _ in range(4)]
                if get_times(int(process._handle),*[ctypes.byref(v) for v in values]):
                    return sum((v.dwHighDateTime<<32)|v.dwLowDateTime for v in values[2:])/10000000
        try:
            while True:
                if time.perf_counter()>=next_compiler_poll:
                    compiler_overlap.update(active_compilers());next_compiler_poll=time.perf_counter()+.5
                sample()
                try:process.wait(timeout=.1);break
                except subprocess.TimeoutExpired:
                    if time.perf_counter()-began>600:raise
            sample();cpu_seconds=cpu();compiler_overlap.update(active_compilers())
            if process.returncode:raise subprocess.CalledProcessError(process.returncode,command)
        except BaseException:
            process.kill();process.wait();raise
    return dict(process_cpu_seconds=cpu_seconds,process_wall_seconds=time.perf_counter()-began,
                peak_private_commit_mib=peak_private,peak_working_set_mib=peak_resident,
                compiler_overlap=sorted(compiler_overlap))


def summarize(rows):
    wall = sorted(float(r['wall_ms']) for r in rows)
    def percentile(p):
        return wall[round((len(wall)-1)*p)]
    return dict(frames=len(wall), fps=1000/statistics.mean(wall),
                median_ms=statistics.median(wall), p95_ms=percentile(.95),
                p99_ms=percentile(.99), worst_ms=max(wall),
                one_percent_low_fps=1000/statistics.mean(wall[-max(1,math.ceil(len(wall)*.01)):]),
                over_16_67=sum(v>1000/60 for v in wall),
                over_33_33=sum(v>1000/30 for v in wall),
                cpu_mean_ms=statistics.mean(float(r['cpu_submit_ms']) for r in rows))


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--exe',type=Path,default=Path('BTD6.exe'))
    parser.add_argument('--reference-exe',type=Path,help='Optional previous build; otherwise disable text cache in the same executable')
    parser.add_argument('--output',type=Path,default=Path('local_extracted/frame-bench'))
    parser.add_argument('--frames',type=int,default=1500)
    parser.add_argument('--warmup',type=int,default=300)
    parser.add_argument('--pairs',type=int,default=3)
    parser.add_argument('--flood',default='Ceramic:4')
    parser.add_argument('--round',type=int,choices=[98,140,200])
    parser.add_argument('--upgrade-case',action='store_true',help='Select Dart, switch to Super at frame 40, buy range at frame 100; measure demand-load frame 101')
    parser.add_argument('--fps-cap',type=int,default=None,help='Override the private FPS cap for pacing measurements')
    parser.add_argument('--degree',type=int,default=20)
    parser.add_argument('--baseline-only',action='store_true',help='Repeat current build without cache-off comparisons')
    parser.add_argument('--suite',action='store_true',help='Run rounds 98/140/200 with 1/2/4 paragons')
    parser.add_argument('--trace-loads',action='store_true',help='Log first-use loading work taking at least 2 ms')
    parser.add_argument('--ab-feature',choices=['frame-pacing','text-cache','pose-reuse','sprite-storage','shadow-textures','map-batching','render-storage','static-maps','background-assets','fallback-models','prediction-retirement'],default='text-cache',help='Feature disabled in same-build reference runs')
    parser.add_argument('--same-build-ab',action='store_true',help='Compare feature off/on when running the suite without a previous executable')
    parser.add_argument('--simulation-snapshot',type=Path,help='Reuse a previously frozen simulation-input directory')
    args=parser.parse_args()
    if args.upgrade_case and (args.suite or args.round or args.frames<103 or args.warmup>=100):parser.error('upgrade case requires no suite/round, frames >= 103, warmup < 100')
    if args.warmup<0 or args.frames<=args.warmup+1 or args.pairs<1:
        parser.error('require pairs >= 1 and frames > warmup + 1 >= 1')
    if args.fps_cap is not None and args.fps_cap<0:parser.error('FPS cap must be nonnegative')
    if not 1<=args.degree<=100:parser.error('degree must be 1 through 100')
    root=Path(__file__).resolve().parents[1]
    exe=args.exe.resolve(); output=args.output.resolve();output.mkdir(parents=True,exist_ok=True)
    snapshot=args.simulation_snapshot.resolve() if args.simulation_snapshot else freeze_inputs(root/'local_extracted',output)
    if args.suite:
        results=[]
        for round_num in (98,140,200):
            destination=output/f'round-{round_num}'
            cmd=[sys.executable,'-I',str(Path(__file__).resolve()),'--exe',str(exe),'--output',str(destination),'--round',str(round_num),'--degree',str(args.degree),'--pairs',str(args.pairs),'--frames',str(max(args.frames,18000)),'--warmup',str(args.warmup)]
            if args.reference_exe:cmd+=['--reference-exe',str(args.reference_exe.resolve())]
            elif not args.same_build_ab:cmd+=['--baseline-only']
            if args.trace_loads:cmd+=['--trace-loads']
            if args.fps_cap is not None:cmd+=['--fps-cap',str(args.fps_cap)]
            cmd+=['--ab-feature',args.ab_feature,'--simulation-snapshot',str(snapshot)]
            subprocess.run(cmd,cwd=root,check=True)
            results.append(json.loads((destination/'results.json').read_text()))
        (output/'suite.json').write_text(json.dumps(results,indent=2),encoding='utf-8')
        lines=['# Paragon round benchmark','', 'Three scenes, fixed degree and freeplay seed, two simulation steps per rendered frame. Figures below are medians of repeated run summaries; worst frame is the maximum across repeats. Initial loading/tower warm-up is excluded, first-use combat stalls are included.','']
        for report in results:
            a=report['aggregate']['candidate'];runs=[r for r in report['runs'] if r['cached']];round_num=report['scenario']['round']
            lines += [f'## Round {round_num}', '',f"- Average: {a['fps']:.1f} FPS; 1% low: {a['one_percent_low_fps']:.1f} FPS.",f"- Frame times: p95 {a['p95_ms']:.2f} ms; p99 {a['p99_ms']:.2f} ms.",f"- Worst individual frame: {max(r['worst_ms'] for r in runs):.2f} ms.",f"- Matching population traces: {report['matching_population_traces']}.", '']
        (output/'summary.md').write_text('\n'.join(lines),encoding='utf-8')
        return
    settings=output/'benchmark-settings.json'
    original=json.loads((snapshot/'settings.json').read_text(encoding='utf-8'))
    settings_data=json.loads(json.dumps(original))
    settings_data.update(map='tutorial',mode='Medium' if args.round or args.upgrade_case else 'Medium Sandbox')
    settings_data.setdefault('advanced',{}).update(fps_cap=0,vsync=False,show_fps=False)
    if args.upgrade_case:settings_data['advanced']['fps_cap']=120
    if args.fps_cap is not None:settings_data['advanced']['fps_cap']=args.fps_cap
    runs=[]; raw={}
    game_data=snapshot/'game/game.json'
    data_hash=hashlib.sha256(game_data.read_bytes()).hexdigest()
    map_data=snapshot/'maps/tutorial/map.json'
    map_hash=hashlib.sha256(map_data.read_bytes()).hexdigest() if map_data.exists() else None
    def check_inputs():
        if hashlib.sha256(game_data.read_bytes()).hexdigest()!=data_hash or (hashlib.sha256(map_data.read_bytes()).hexdigest() if map_data.exists() else None)!=map_hash:
            raise RuntimeError('Frozen simulation inputs changed; no timing result accepted')
        if json.loads(settings.read_text())!=settings_data:raise RuntimeError('Benchmark preferences changed during capture')
    try:
        settings.write_text(json.dumps(settings_data),encoding='utf-8')
        for pair in range(args.pairs):
            for enabled in ([True] if args.baseline_only else ([False,True] if pair%2==0 else [True,False])):
                label=f'{pair+1}-'+('cached' if enabled else 'reference')
                env={k:v for k,v in os.environ.items() if not k.startswith('RRBTD6_')}
                env.update(RRBTD6_WINDOW='1600x900',RRBTD6_AUTOTEST=str(output/(label+'.png')),
                           RRBTD6_FRAME_BENCH=str(output/(label+'.csv')),
                           RRBTD6_FRAMES=str(args.frames),RRBTD6_BENCH_STEPS='2',
                           RRBTD6_FLOOD=args.flood,RRBTD6_TOWERS='DartMonkey',
                           RRBTD6_CASH='5000',RRBTD6_NOSELECT='1',
                           RRBTD6_BENCH_SIM_DIR=str(snapshot),RRBTD6_BENCH_SETTINGS=str(settings))
                if args.round:
                    env.pop('RRBTD6_FLOOD',None)
                    env.update(RRBTD6_BENCH_ROUND=str(args.round),RRBTD6_BENCH_DEGREE=str(args.degree),RRBTD6_BENCH_WARMUP=str(args.warmup))
                if args.upgrade_case:
                    env.pop('RRBTD6_FLOOD',None);env.pop('RRBTD6_NOSELECT',None)
                    env.update(RRBTD6_TOWERS='DartMonkey,SuperMonkey',RRBTD6_PLACE='300,520',RRBTD6_PATHS='',RRBTD6_CASH='1000000',RRBTD6_UPGRADE_CASE='1:40:1:100')
                if args.trace_loads:env['RRBTD6_TIMING']='1'
                if not enabled and not args.reference_exe:
                    feature_env={'frame-pacing':'RRBTD6_LEGACY_FRAME_PACING','prediction-retirement':'RRBTD6_NO_PREDICTIVE_RETIRE','fallback-models':'RRBTD6_KEEP_FALLBACK_MODELS','background-assets':'RRBTD6_NO_BACKGROUND_ASSETS','pose-reuse':'RRBTD6_NO_POSE_REUSE','text-cache':'RRBTD6_NO_TEXT_CACHE','sprite-storage':'RRBTD6_KEEP_SPRITE_DUPLICATES','shadow-textures':'RRBTD6_NO_SHARED_SHADOWS','map-batching':'RRBTD6_NO_MAP_BATCHING','render-storage':'RRBTD6_LEGACY_RENDER_STORAGE','static-maps':'RRBTD6_NO_STATIC_MAP_GPU'}
                    env[feature_env[args.ab_feature]]='1'
                run_exe=exe if enabled or not args.reference_exe else args.reference_exe.resolve()
                startup=None; flags=0
                if os.name=='nt':
                    startup=subprocess.STARTUPINFO();startup.dwFlags|=subprocess.STARTF_USESHOWWINDOW
                    startup.wShowWindow=0;flags=subprocess.CREATE_NO_WINDOW
                for attempt in range(1,6):
                    wait_for_compilers()
                    check_inputs()
                    with (output/(label+'.log')).open('w',encoding='utf-8') as log:
                        process_metrics=run_measured([str(run_exe)],cwd=root,env=env,stdout=log,stderr=subprocess.STDOUT,
                                       startupinfo=startup,creationflags=flags)
                    check_inputs()
                    expected_marker='frame-bench-inputs: isolated mode='+settings_data['mode']
                    if expected_marker not in (output/(label+'.log')).read_text():
                        raise RuntimeError('Executable lacks isolated-input support or used the wrong mode; rebuild it before comparing')
                    if not process_metrics['compiler_overlap']:break
                    print('Rejected compiler overlap:',label,process_metrics['compiler_overlap'],flush=True)
                    for suffix in ('.log','.csv','.png'):
                        path=output/(label+suffix)
                        if path.exists():path.replace(output/(label+f'-compiler-overlap-{attempt}'+suffix))
                else:raise RuntimeError('Five captures overlapped compilation; no timing result accepted')
                with (output/(label+'.csv')).open(newline='') as f:
                    rows=[r for r in csv.DictReader(f) if int(r['frame'])>args.warmup]
                if args.round:
                    expected={98:1,140:2,200:4}[args.round]
                    if f'round-bench: completed round {args.round} ' not in (output/(label+'.log')).read_text():
                        raise RuntimeError('Requested round did not complete; increase --frames')
                    if not rows or any(int(r['round'])!=args.round or int(r['paragons'])!=expected or r['round_active']!='true' for r in rows):
                        raise RuntimeError('Wrong round, paragon count, or inactive gameplay in measured scene')
                elif len(rows)!=args.frames-args.warmup-1:
                    raise RuntimeError(f'Incomplete gameplay capture: {len(rows)} frames; no timing comparison accepted')
                raw[label]=rows
                dimensions=struct.unpack('>II',(output/(label+'.png')).read_bytes()[16:24])
                run=dict(label=label,cached=enabled,framebuffer_pixels=dimensions,peak_bloons=max(int(r['bloons']) for r in rows),peak_projectiles=max(int(r['projectiles']) for r in rows),executable_sha256=hashlib.sha256(run_exe.read_bytes()).hexdigest(),**summarize(rows),**process_metrics);runs.append(run)
                if args.upgrade_case:
                    if 'upgrade-case: upgraded 1 path 1 frame 100' not in (output/(label+'.log')).read_text():raise RuntimeError('Upgrade fixture did not run')
                    demand=next(r for r in rows if int(r['frame'])==101)
                    run.update(upgrade_cpu_ms=float(demand['cpu_submit_ms']),upgrade_wall_ms=float(demand['wall_ms']))
                print(json.dumps(run),flush=True)
        # Fail visibly if the scene populations differed. Timings alone cannot prove simulation identity.
        workloads=[[(r['frame'],r['bloons'],r['projectiles'],r['towers']) for r in rows] for rows in raw.values()]
        same=all(w==workloads[0] for w in workloads)
        aggregate={}
        for enabled,name in [(False,'reference'),(True,'candidate')]:
            group=[r for r in runs if r['cached']==enabled]
            if not group:continue
            aggregate[name]={k:statistics.median(r[k] for r in group) for k in ('fps','one_percent_low_fps','median_ms','p95_ms','p99_ms','worst_ms','cpu_mean_ms')}
            for k in ('process_cpu_seconds','process_wall_seconds','peak_private_commit_mib','peak_working_set_mib'):
                if all(r[k] is not None for r in group):aggregate[name][k]=statistics.median(r[k] for r in group)
            if args.upgrade_case:
                for k in ('upgrade_cpu_ms','upgrade_wall_ms'):aggregate[name][k]=statistics.median(r[k] for r in group)
        if 'reference' in aggregate:aggregate['fps_change_percent']=(aggregate['candidate']['fps']/aggregate['reference']['fps']-1)*100
        report=dict(executable=str(exe),sha256=hashlib.sha256(exe.read_bytes()).hexdigest(),
                    game_data_sha256=data_hash,map_data_sha256=map_hash,simulation_snapshot=str(snapshot),reference_sha256=hashlib.sha256(args.reference_exe.resolve().read_bytes()).hexdigest() if args.reference_exe else None,
                    scenario={k:str(v) if isinstance(v,Path) else v for k,v in vars(args).items()},
                    settings=settings_data,runs=runs,aggregate=aggregate,matching_population_traces=same)
        (output/'results.json').write_text(json.dumps(report,indent=2),encoding='utf-8')
        print('Population traces match:',same,flush=True)
        print('Aggregate:',json.dumps(aggregate),flush=True)
    finally:
        # Keep reproducible private preferences, including after a failed capture.
        # Live user settings are never written by this runner.
        settings.write_text(json.dumps(settings_data),encoding='utf-8')


if __name__=='__main__':main()
