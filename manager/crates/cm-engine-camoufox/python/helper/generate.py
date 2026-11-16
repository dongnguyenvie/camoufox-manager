"""`generate --os win|mac|lin`: build the pinned machine once (phase 4 owns pinning details)."""

from common import OS_NAMES, decode_chunks, emit


def cmd_generate(args):
    from camoufox.utils import launch_options

    # headless=False: screen/window size are capped to this machine's real screen.
    # geoip=False: geo/timezone are applied by the app at each launch, not baked into the machine.
    opts = launch_options(os=OS_NAMES[args.os], headless=False, geoip=False, i_know_what_im_doing=True)
    config = decode_chunks(opts["env"], "CAMOU_CONFIG_")
    # addons are paths inside this install; launch adds them back.
    config.pop("addons", None)
    emit({"config": config})
