# Copying the logic from z3's preamble in z3core.py to ensure we have the same linking behavior
# This code taken from here: https://github.com/Z3Prover/z3/blob/84a5303def9bea552de91bd68089b979a0bf87d2/scripts/update_api.py

import atexit
import sys, os
import contextlib
import ctypes
from os import RTLD_GLOBAL, RTLD_NOW

from z3 import Z3Exception

if sys.version_info >= (3, 9):
    import importlib.resources as importlib_resources
else:
    import importlib_resources

_file_manager = contextlib.ExitStack()
atexit.register(_file_manager.close)
_ext = 'dll' if sys.platform in ('win32', 'cygwin') else 'dylib' if sys.platform == 'darwin' else 'so'
_lib = None
_z3_lib_resource = importlib_resources.files('z3').joinpath('lib')
_z3_lib_resource_path = _file_manager.enter_context(
    importlib_resources.as_file(_z3_lib_resource)
)
_default_dirs = ['.',
                 os.path.dirname(os.path.abspath(__file__)),
                 _z3_lib_resource_path,
                 os.path.join(sys.prefix, 'lib'),
                 None]
_all_dirs = []
# search the default dirs first
_all_dirs.extend(_default_dirs)

if sys.version < '3':
    import __builtin__
    if hasattr(__builtin__, "Z3_LIB_DIRS"):
        _all_dirs = __builtin__.Z3_LIB_DIRS
else:
    import builtins
    if hasattr(builtins, "Z3_LIB_DIRS"):
        _all_dirs = builtins.Z3_LIB_DIRS

for v in ('Z3_LIBRARY_PATH', 'PATH', 'PYTHONPATH'):
    if v in os.environ:
        lp = os.environ[v];
        lds = lp.split(';') if sys.platform in ('win32') else lp.split(':')
        _all_dirs.extend(lds)

_failures = []
for d in _all_dirs:
    try:
        d = os.path.realpath(d)
        if os.path.isdir(d):
            d_dir = d
            d = os.path.join(d, 'libz3.%s' % _ext)
            if os.path.isfile(d):
                _lib = ctypes.CDLL(d, mode=RTLD_GLOBAL | RTLD_NOW)
                # change: we need to add this to this process's LD_LIBRARY_PATH
                break
    except Exception as e:
        _failures += [e]
        pass

if _lib is None:
    # If all else failed, ask the system to find it.
    try:
        _lib = ctypes.CDLL('libz3.%s' % _ext)
    except Exception as e:
        _failures += [e]
        pass

if _lib is None:
    print("Could not find libz3.%s; consider adding the directory containing it to" % _ext)
    print("  - your system's PATH environment variable,")
    print("  - the Z3_LIBRARY_PATH environment variable, or ")
    print("  - to the custom Z3_LIB_DIRS Python-builtin before importing the z3 module, e.g. via")
    if sys.version < '3':
        print("    import __builtin__")
        print("    __builtin__.Z3_LIB_DIRS = [ '/path/to/z3/lib/dir' ] # directory containing libz3.%s" % _ext)
    else:
        print("    import builtins")
        print("    builtins.Z3_LIB_DIRS = [ '/path/to/z3/lib/dir' ] # directory containing libz3.%s" % _ext)
    print(_failures)
    raise Z3Exception("libz3.%s not found." % _ext)
