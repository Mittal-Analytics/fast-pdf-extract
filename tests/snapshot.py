import os
import webbrowser
from pathlib import Path


class SnapshotChanged(Exception):
    pass


def compare_snapshot(html, save_path):
    is_bytes = isinstance(html, bytes)
    write_mode = "wb" if is_bytes else "w"
    read_mode = "rb" if is_bytes else "r"

    # save and re-read the html to ensure save encoding differences
    compare_path = Path(f"{save_path}_")
    compare_path.write_bytes(html) if is_bytes else compare_path.write_text(html)
    html = compare_path.read_bytes() if is_bytes else compare_path.read_text()
    compare_path.unlink()

    existing = ""
    if os.path.isfile(save_path):
        with open(save_path, read_mode) as tmp:
            existing = tmp.read()

    if existing == html:
        return True

    with open(save_path, write_mode) as tmp:
        tmp.write(html)
    abs_path = "file://{}".format(os.path.abspath(save_path))
    webbrowser.open(abs_path)
    raise SnapshotChanged("Snapshot has been updated")
