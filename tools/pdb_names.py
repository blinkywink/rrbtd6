"""Name RVAs in a Windows exe through dbghelp (SymFromAddr). Usage: from pdb_names import Namer."""
import ctypes, ctypes.wintypes as wt, os

class SYMBOL_INFOW(ctypes.Structure):
    _fields_ = [("SizeOfStruct", wt.ULONG), ("TypeIndex", wt.ULONG), ("Reserved", ctypes.c_ulonglong * 2),
                ("Index", wt.ULONG), ("Size", wt.ULONG), ("ModBase", ctypes.c_ulonglong), ("Flags", wt.ULONG),
                ("Value", ctypes.c_ulonglong), ("Address", ctypes.c_ulonglong), ("Register", wt.ULONG),
                ("Scope", wt.ULONG), ("Tag", wt.ULONG), ("NameLen", wt.ULONG), ("MaxNameLen", wt.ULONG),
                ("Name", wt.WCHAR * 1024)]

class Namer:
    BASE = 0x10000000
    def __init__(self, exe):
        self.d = ctypes.WinDLL("dbghelp.dll")
        self.h = wt.HANDLE(0x7654)
        self.d.SymSetOptions(0x2 | 0x10)  # UNDNAME | LOAD_LINES
        if not self.d.SymInitializeW(self.h, ctypes.c_wchar_p(os.path.dirname(exe)), False):
            raise OSError("SymInitialize")
        self.d.SymLoadModuleExW.restype = ctypes.c_ulonglong
        base = self.d.SymLoadModuleExW(self.h, None, ctypes.c_wchar_p(exe), None, ctypes.c_ulonglong(self.BASE), 0, None, 0)
        if not base:
            raise OSError("SymLoadModuleEx")
        self.cache = {}
    def name(self, rva):
        if rva in self.cache:
            return self.cache[rva]
        s = SYMBOL_INFOW(); s.SizeOfStruct = 88; s.MaxNameLen = 1024
        disp = ctypes.c_ulonglong(0)
        ok = self.d.SymFromAddrW(self.h, ctypes.c_ulonglong(self.BASE + rva), ctypes.byref(disp), ctypes.byref(s))
        n = s.Name if ok else None
        self.cache[rva] = n
        return n
