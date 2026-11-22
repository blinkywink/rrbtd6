"""A small sampling profiler for Windows that needs no admin rights: runs a command, suspends its
busiest thread ~1000 times a second, walks the stack with dbghelp's StackWalk64 and resolves names
from the exe's PDB. Prints self and inclusive shares per function.

Usage: python -I tools/sample_prof.py [--top N] [--hz H] -- EXE ARGS...
"""
import ctypes, ctypes.wintypes as wt, collections, subprocess, sys, time

k32 = ctypes.WinDLL("kernel32", use_last_error=True)
dbg = ctypes.WinDLL("dbghelp", use_last_error=True)
psapi = ctypes.WinDLL("psapi", use_last_error=True)

THREAD_ALL = 0x1FFFFF
CONTEXT_FULL = 0x10000B


class M128A(ctypes.Structure):
    _fields_ = [("Low", ctypes.c_ulonglong), ("High", ctypes.c_longlong)]


class CONTEXT(ctypes.Structure):
    _pack_ = 16
    _fields_ = [("P1Home", ctypes.c_ulonglong), ("P2Home", ctypes.c_ulonglong), ("P3Home", ctypes.c_ulonglong),
                ("P4Home", ctypes.c_ulonglong), ("P5Home", ctypes.c_ulonglong), ("P6Home", ctypes.c_ulonglong),
                ("ContextFlags", wt.DWORD), ("MxCsr", wt.DWORD),
                ("SegCs", wt.WORD), ("SegDs", wt.WORD), ("SegEs", wt.WORD), ("SegFs", wt.WORD), ("SegGs", wt.WORD),
                ("SegSs", wt.WORD), ("EFlags", wt.DWORD),
                ("Dr0", ctypes.c_ulonglong), ("Dr1", ctypes.c_ulonglong), ("Dr2", ctypes.c_ulonglong),
                ("Dr3", ctypes.c_ulonglong), ("Dr6", ctypes.c_ulonglong), ("Dr7", ctypes.c_ulonglong),
                ("Rax", ctypes.c_ulonglong), ("Rcx", ctypes.c_ulonglong), ("Rdx", ctypes.c_ulonglong),
                ("Rbx", ctypes.c_ulonglong), ("Rsp", ctypes.c_ulonglong), ("Rbp", ctypes.c_ulonglong),
                ("Rsi", ctypes.c_ulonglong), ("Rdi", ctypes.c_ulonglong), ("R8", ctypes.c_ulonglong),
                ("R9", ctypes.c_ulonglong), ("R10", ctypes.c_ulonglong), ("R11", ctypes.c_ulonglong),
                ("R12", ctypes.c_ulonglong), ("R13", ctypes.c_ulonglong), ("R14", ctypes.c_ulonglong),
                ("R15", ctypes.c_ulonglong), ("Rip", ctypes.c_ulonglong),
                ("FltSave", ctypes.c_byte * 512), ("VectorRegister", M128A * 26), ("VectorControl", ctypes.c_ulonglong),
                ("DebugControl", ctypes.c_ulonglong), ("LastBranchToRip", ctypes.c_ulonglong),
                ("LastBranchFromRip", ctypes.c_ulonglong), ("LastExceptionToRip", ctypes.c_ulonglong),
                ("LastExceptionFromRip", ctypes.c_ulonglong)]


class ADDRESS64(ctypes.Structure):
    _fields_ = [("Offset", ctypes.c_ulonglong), ("Segment", wt.WORD), ("Mode", ctypes.c_int)]


class STACKFRAME64(ctypes.Structure):
    _fields_ = [("AddrPC", ADDRESS64), ("AddrReturn", ADDRESS64), ("AddrFrame", ADDRESS64), ("AddrStack", ADDRESS64),
                ("AddrBStore", ADDRESS64), ("FuncTableEntry", ctypes.c_void_p), ("Params", ctypes.c_ulonglong * 4),
                ("Far", wt.BOOL), ("Virtual", wt.BOOL), ("Reserved", ctypes.c_ulonglong * 3), ("KdHelp", ctypes.c_byte * 112)]


class SYMBOL_INFOW(ctypes.Structure):
    _fields_ = [("SizeOfStruct", wt.ULONG), ("TypeIndex", wt.ULONG), ("Reserved", ctypes.c_ulonglong * 2),
                ("Index", wt.ULONG), ("Size", wt.ULONG), ("ModBase", ctypes.c_ulonglong), ("Flags", wt.ULONG),
                ("Value", ctypes.c_ulonglong), ("Address", ctypes.c_ulonglong), ("Register", wt.ULONG),
                ("Scope", wt.ULONG), ("Tag", wt.ULONG), ("NameLen", wt.ULONG), ("MaxNameLen", wt.ULONG),
                ("Name", wt.WCHAR * 1024)]


class THREADENTRY32(ctypes.Structure):
    _fields_ = [("dwSize", wt.DWORD), ("cntUsage", wt.DWORD), ("th32ThreadID", wt.DWORD), ("th32OwnerProcessID", wt.DWORD),
                ("tpBasePri", wt.LONG), ("tpDeltaPri", wt.LONG), ("dwFlags", wt.DWORD)]


def threads(pid):
    snap = k32.CreateToolhelp32Snapshot(0x4, 0)
    te = THREADENTRY32(); te.dwSize = ctypes.sizeof(te)
    out = []
    ok = k32.Thread32First(snap, ctypes.byref(te))
    while ok:
        if te.th32OwnerProcessID == pid:
            out.append(te.th32ThreadID)
        ok = k32.Thread32Next(snap, ctypes.byref(te))
    k32.CloseHandle(snap)
    return out


def thread_cpu(h):
    c, e, kt, ut = (wt.FILETIME() for _ in range(4))
    k32.GetThreadTimes(h, ctypes.byref(c), ctypes.byref(e), ctypes.byref(kt), ctypes.byref(ut))
    return (ut.dwHighDateTime << 32) | ut.dwLowDateTime


def main():
    args = sys.argv[1:]
    top, hz = 40, 1000
    while args and args[0] != "--":
        if args[0] == "--top":
            top = int(args[1]); args = args[2:]
        elif args[0] == "--hz":
            hz = int(args[1]); args = args[2:]
        else:
            break
    if args and args[0] == "--":
        args = args[1:]
    import os
    args[0] = os.path.abspath(args[0])
    ctypes.WinDLL("winmm").timeBeginPeriod(1)
    proc = subprocess.Popen(args, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    k32.OpenProcess.restype = wt.HANDLE
    hp = k32.OpenProcess(0x1F0FFF, False, proc.pid)
    dbg.SymSetOptions(0x2 | 0x10)
    time.sleep(0.3)
    dbg.SymInitializeW(wt.HANDLE(hp), None, True)
    mods = (ctypes.c_void_p * 1)()
    need = wt.DWORD()
    psapi.EnumProcessModules(wt.HANDLE(hp), mods, ctypes.sizeof(mods), ctypes.byref(need))
    exe_base = mods[0] or 0

    dbg.SymFunctionTableAccess64.restype = ctypes.c_void_p
    dbg.SymGetModuleBase64.restype = ctypes.c_ulonglong
    k32.OpenThread.restype = wt.HANDLE
    handles = {}
    stacks = collections.Counter()
    nsamp = 0
    period = 1.0 / hz
    last_refresh = 0
    while proc.poll() is None:
        now = time.perf_counter()
        if now - last_refresh > 0.5:
            for tid in threads(proc.pid):
                if tid not in handles:
                    h = k32.OpenThread(THREAD_ALL, False, tid)
                    if h:
                        handles[tid] = [h, 0]
            last_refresh = now
            dbg.SymRefreshModuleList(wt.HANDLE(hp))
        # The busiest thread since the last sample.
        best = None
        for tid, v in list(handles.items()):
            t = thread_cpu(v[0])
            d = t - v[1]
            v[1] = t
            if best is None or d > best[1]:
                best = (tid, d)
        if not best or best[1] == 0:
            time.sleep(period)
            continue
        h = handles[best[0]][0]
        if k32.SuspendThread(h) == 0xFFFFFFFF:
            continue
        ctx = CONTEXT(); ctx.ContextFlags = CONTEXT_FULL
        frames = []
        if k32.GetThreadContext(h, ctypes.byref(ctx)):
            sf = STACKFRAME64()
            sf.AddrPC.Offset, sf.AddrPC.Mode = ctx.Rip, 3
            sf.AddrFrame.Offset, sf.AddrFrame.Mode = ctx.Rbp, 3
            sf.AddrStack.Offset, sf.AddrStack.Mode = ctx.Rsp, 3
            for _ in range(64):
                if not dbg.StackWalk64(0x8664, wt.HANDLE(hp), wt.HANDLE(h), ctypes.byref(sf), ctypes.byref(ctx), None,
                                       dbg.SymFunctionTableAccess64, dbg.SymGetModuleBase64, None):
                    break
                if sf.AddrPC.Offset == 0:
                    break
                frames.append(sf.AddrPC.Offset)
        k32.ResumeThread(h)
        if frames:
            stacks[tuple(frames)] += 1
            nsamp += 1
        time.sleep(period)
    # The exe's load base, so its frames resolve offline through its PDB (live lookups miss it).
    sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
    from pdb_names import Namer
    namer = Namer(args[0])
    names = {}

    def name(a):
        if a not in names and exe_base and 0 <= a - exe_base < 0x10000000:
            names[a] = namer.name(a - exe_base) or hex(a - exe_base)
        if a not in names:
            s = SYMBOL_INFOW(); s.SizeOfStruct = 88; s.MaxNameLen = 1024
            disp = ctypes.c_ulonglong(0)
            names[a] = s.Name if dbg.SymFromAddrW(wt.HANDLE(hp), ctypes.c_ulonglong(a), ctypes.byref(disp), ctypes.byref(s)) else hex(a)
        return names[a]
    selfc, incl = collections.Counter(), collections.Counter()
    for st, c in stacks.items():
        selfc[name(st[0])] += c
        for n in {name(a) for a in st}:
            incl[n] += c
    print(f"{nsamp} samples")
    print("-- self")
    for k, c in selfc.most_common(top):
        print(f"{100 * c / max(nsamp, 1):5.1f}%  {k[:150]}")
    # Who calls the top self-time functions (the first frame of ours above them).
    print("-- callers")
    for k, c in selfc.most_common(12):
        callers = collections.Counter()
        for st, n in stacks.items():
            if name(st[0]) != k:
                continue
            for a in st[1:]:
                nm = name(a)
                if nm != k and "btd6_game" in nm:
                    callers[nm] += n
                    break
        print(f"{k[:90]}")
        for nm, n in callers.most_common(4):
            print(f"    {100 * n / max(nsamp, 1):5.1f}%  {nm[:120]}")
    print("-- inclusive")
    for k, c in incl.most_common(top):
        print(f"{100 * c / max(nsamp, 1):5.1f}%  {k[:150]}")


if __name__ == "__main__":
    main()
