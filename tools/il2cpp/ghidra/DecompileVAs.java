// Decompile the IL2CPP methods listed in a file (one VA in hex per line) to C, one file each.
//
// Run through tools/il2cpp/decompile.py, which writes the VA list and the names file and calls
// analyzeHeadless. Arguments: <vas file> <names file (va<TAB>name per line)> <out dir>.
// Functions are created at the given addresses without whole-program analysis (that takes
// hours on GameAssembly.dll); known method addresses get their Type::Method names so calls read.
//@category rrbtd6

import java.io.*;
import java.nio.file.*;
import java.util.*;

import ghidra.app.cmd.disassemble.DisassembleCommand;
import ghidra.app.cmd.function.CreateFunctionCmd;
import ghidra.app.decompiler.*;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.*;
import ghidra.program.model.listing.*;
import ghidra.program.model.symbol.*;

public class DecompileVAs extends GhidraScript {
    @Override
    public void run() throws Exception {
        String[] args = getScriptArgs();
        List<String> vas = Files.readAllLines(Paths.get(args[0]));
        Path out = Paths.get(args[2]);
        Files.createDirectories(out);
        AddressSpace space = currentProgram.getAddressFactory().getDefaultAddressSpace();
        SymbolTable st = currentProgram.getSymbolTable();

        // Names: only once per program (marker label at the image base).
        Address base = currentProgram.getImageBase();
        if (st.getSymbols("rrbtd6_names_loaded").hasNext() == false) {
            int n = 0;
            for (String line : Files.readAllLines(Paths.get(args[1]))) {
                String[] p = line.split("\t", 2);
                if (p.length < 2) continue;
                Address a = space.getAddress(Long.parseUnsignedLong(p[0], 16));
                try {
                    st.createLabel(a, sanitize(p[1]), SourceType.IMPORTED);
                    n++;
                } catch (Exception e) {
                    // duplicate or bad name: skip
                }
            }
            st.createLabel(base, "rrbtd6_names_loaded", SourceType.USER_DEFINED);
            println("labels: " + n);
        }

        DecompInterface di = new DecompInterface();
        di.openProgram(currentProgram);
        for (String v : vas) {
            v = v.trim();
            if (v.isEmpty()) continue;
            Address a = space.getAddress(Long.parseUnsignedLong(v.replace("0x", ""), 16));
            Function f = getFunctionAt(a);
            if (f == null) {
                new DisassembleCommand(a, null, true).applyTo(currentProgram, monitor);
                new CreateFunctionCmd(a).applyTo(currentProgram, monitor);
                f = getFunctionAt(a);
            }
            if (f == null) {
                println("no function at " + v);
                continue;
            }
            // Callees as functions too, so their names and signatures show.
            for (Function c : f.getCalledFunctions(monitor)) {
                // already created by CreateFunctionCmd's flow following, nothing to do
            }
            DecompileResults r = di.decompileFunction(f, 120, monitor);
            String c = r.decompileCompleted() ? r.getDecompiledFunction().getC() : ("// failed: " + r.getErrorMessage());
            Files.writeString(out.resolve(v + ".c"), c);
        }
        di.dispose();
    }

    static String sanitize(String s) {
        StringBuilder b = new StringBuilder();
        for (char ch : s.toCharArray()) {
            b.append(Character.isLetterOrDigit(ch) || ch == '_' || ch == ':' || ch == '.' ? ch : '_');
        }
        return b.toString().replace("::", "__");
    }
}
