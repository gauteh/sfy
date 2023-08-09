from . import axl, signal, hub, xr


def version():
    """
    Return git version if available.
    """
    import os.path
    from subprocess import check_output, DEVNULL

    path = os.path.dirname(__file__)
    args = [
        "git", "-C", path, "describe", "--tags", "--abbrev=7", "--dirty",
        "--broken", "--always",
    ]

    try:
        version = check_output(args, cwd=path, stderr=DEVNULL).decode().strip()
        return version
    except:
        return None
