"""`extensions install --user-data DIR XPI...` — phase 9 implements this.

Installs Firefox extensions (.xpi) into a profile before launch.
"""

from common import emit


def cmd_extensions(args):
    emit({"error": "extensions: not implemented yet", "code": "not_implemented"})
    raise SystemExit(1)
