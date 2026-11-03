"""`zig cc` as the C compiler and linker for aarch64-apple-ios, for tools/build_ios.sh.

cargo-zigbuild doesn't do iOS, but Zig can, given Apple's SDK. Cargo and the cc crate call this
through the .cmd wrappers build_ios.sh writes, with the flags they'd give Apple's clang; the ones Zig
doesn't take are dropped.

Needs ZIG (zig's path), SDKROOT and IPHONEOS_DEPLOYMENT_TARGET.
The SDK goes in by -I, -F and -L: with --sysroot, Zig on Windows glues the sysroot onto absolute -L paths.
"""
import os
import re
import subprocess
import sys
import tempfile

# Flags with a value in the next argument that Zig gets from -target or --sysroot instead, or doesn't need.
DROP_WITH_VALUE = {"-target", "-arch", "-isysroot", "-syslibroot", "-platform_version", "-Wl,-exported_symbols_list"}
DROP = {"-dead_strip", "-nodefaultlibs", "-Wl,-dead_strip"}
DROP_PREFIX = ("-miphoneos-version-min", "-mios-version-min", "--target=")


def expand(args):
    """Rustc puts a long command in an @file: one argument a line, with backslash escapes."""
    for arg in args:
        if not (arg.startswith("@") and os.path.isfile(arg[1:])):
            yield arg
            continue
        for line in open(arg[1:], encoding="utf-8").read().splitlines():
            line = line.strip()
            if line.startswith('"') and line.endswith('"'):
                line = line[1:-1]
            yield re.sub(r"\\(.)", r"\1", line)


def main():
    sdk = os.environ["SDKROOT"]
    version = os.environ.get("IPHONEOS_DEPLOYMENT_TARGET", "15.0")
    args, skip = [], False
    for arg in expand(sys.argv[1:]):
        if skip:
            skip = False
        elif arg in DROP_WITH_VALUE:
            skip = True
        elif arg not in DROP and not arg.startswith(DROP_PREFIX):
            args.append(arg)
    command = [os.environ["ZIG"], "cc", "-target", f"aarch64-ios.{version}",
               f"-I{sdk}/usr/include", f"-F{sdk}/System/Library/Frameworks", f"-L{sdk}/usr/lib"]
    if sum(len(arg) + 3 for arg in args) < 8000:
        sys.exit(subprocess.run(command + args).returncode)
    # Too long for a command line, so it goes back in a file for Zig.
    with tempfile.NamedTemporaryFile("w", suffix=".rsp", delete=False, encoding="utf-8") as stream:
        for arg in args:
            stream.write('"' + arg.replace("\\", "/") + '"\n')
    try:
        code = subprocess.run(command + ["@" + stream.name]).returncode
    finally:
        os.unlink(stream.name)
    sys.exit(code)


main()
