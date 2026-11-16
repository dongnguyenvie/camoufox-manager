"""`cookies export|import --user-data DIR --file FILE` — phase 5 implements this.

Reads/writes cookies of a CLOSED profile (Firefox cookies.sqlite in the user-data dir).
"""

from common import emit


def cmd_cookies(args):
    emit({"error": "cookies: not implemented yet", "code": "not_implemented"})
    raise SystemExit(1)
