"""
Contains utilities for getting user input.
"""

from dataclasses import dataclass
import os
from pathlib import Path
import time
import urllib.request
from typing import Any, Iterable, Optional

from . import sh
from . import log
from .log import Color

__confirm_auto_answer = None


def set_confirm_auto_answer(auto_answer: Optional[str]) -> None:
    """
    When set to a value other than `None`, `confirm()` will be replied to
    automatically with this value instead of prompting the user.
    """

    global __confirm_auto_answer
    __confirm_auto_answer = auto_answer


def confirm(*args: Any, sep: Optional[str] = " ") -> bool:
    """
    Log a message and await a "yes"/"no" from the user.
    """

    print(f"{Color.CONFIRM}CONFIRM{Color.RESET}: ", end="")
    print(*args, sep=sep, end="")
    print(f" ({Color.CONFIRM}y{Color.RESET}/n): {Color.CONFIRM}", end="")

    if __confirm_auto_answer is None:
        try:
            response = input().strip().lower()
        except KeyboardInterrupt:
            print(f"{Color.RESET}({Color.ERROR}canceled{Color.RESET})")
            response = ""

        print(f"{Color.RESET}", end="", flush=True)
    else:
        response = __confirm_auto_answer
        print(f"{__confirm_auto_answer}{Color.RESET} (auto)")

    return response in ("y", "yes")


def action_needed(*args: Any, sep: Optional[str] = " ") -> None:
    """
    Log a message and wait for the user to hit enter, allowing them to
    optionally run a command.
    """

    print(f"{Color.ACTION_NEEDED}MANUAL ACTION NEEDED{Color.RESET}: ", end="")
    print(*args, sep=sep, end="")
    print(
        f" (press [{Color.ACTION_NEEDED}ENTER{Color.RESET}] if you have "
        + "completed the action manually or enter a shell command to run): "
        + f"{Color.ACTION_NEEDED}",
        end="",
    )

    output_is_terminal = Color.RESET != ""

    try:
        response = input()
    except KeyboardInterrupt:
        print(f"{Color.RESET}({Color.ERROR}canceled{Color.RESET})")
        log.fatal("No input supplied.", include_run_again_msg=False)

    if not output_is_terminal:
        print(f"{response}{Color.RESET} (auto)")
    print(f"{Color.RESET}", end="", flush=True)

    if len(response) != 0 and not response.isspace():
        sh.run_cmd(response, shell=True)


def ask_to_clear_up_path(path: str) -> None:
    """
    If there is an object at the provided path, the user is asked to move/remove
    it (with an option to have it removed automatically). If this function
    returns, the path has been cleared.
    """

    if not os.path.exists(path):
        return

    if not confirm(
        f"An object already exists at `{path}`. "
        + "It must be moved/removed. Remove it?"
    ):
        if os.path.exists(path):
            log.fatal(
                f"Can't continue while an object at `{path}` still exists."
            )
        else:
            log.info(f"Object at `{path}` has moved. Continuing...")

    if sh.rm_path(path, allow_missing=True):
        log.warning(f"Removed `{path}`.")
    else:
        log.info(f"Nothing to remove anymore at `{path}`.")


class DownloadRejectedException(Exception):
    """
    Raised if the user rejects the download of a file.
    """


class DownloadCanceledException(DownloadRejectedException):
    """
    Raised if the user cancels the download of a file.
    """


def ask_to_download(
    url: str,
    dest_path: str,
    download_reason: Optional[str] = None,
    *,
    require_download_completes: bool = True,
    non_fatal: bool = False,
    # region Output Format Parameters
    show_bar: bool = True,
    bar_inner_width: int = 18,
    bar_inner_filled: str = "#",
    bar_inner_empty: str = " ",
    bar_inner_style: Optional[Iterable[Color]] = (Color.INFO,),
    bar_prefix: str = "[",
    bar_suffix: str = "]",
    show_percent: bool = True,
    show_bytes_progress: bool = True,
    # endregion
) -> bool:
    """
    Download a (potentially large) file from the internet.

    If `require_download_completes` is `True`, this function will always return
    `True`. Otherwise, `False` can be returned if the download is rejected or
    canceled.

    When `require_download_completes` is `True` and `non_fatal` is `False`, a
    rejection will cause `DownloadRejectedException` to be raised. A
    cancellation will cause `DownloadCanceledException` (a sub-class of
    `DownloadRejectedException`) to be raised. If anything else goes wrong, the
    exception that caused the download to fail will be raised. If `non_fatal` is
    `False`, the function will exit instead of raising any exceptions.

    If outputting to a terminal, download progress will continually be redrawn
    over the same line. Otherwise it will write to a new line repeatedly (every
    few seconds). If all `show_*` parameters are `False`, no progress will be
    shown.

    To enable download canceling, this function may catch a `KeyboardInterrupt`
    exception.
    """

    overwrite_output = Color.enabled()

    if not confirm(
        f"Allow download from `{log.Color.INFO}{url}{log.Color.RESET}`?"
        + (" " + download_reason if download_reason else "")
    ):
        if require_download_completes:
            if non_fatal:
                raise DownloadRejectedException()
            log.fatal("The download is required to continue.")
        return False

    try:
        os.makedirs(Path(dest_path).parent, exist_ok=True)
    except Exception as e:
        log.fatal(f"Failed to create directories to support `{dest_path}`.")
    ask_to_clear_up_path(dest_path)

    def size_with_unit(bytes: int, format_str: str = "{size:.1f}{unit}") -> str:
        size = float(bytes)

        possible_units = ["TiB", "GiB", "MiB", "KiB", "B"]
        unit = possible_units[-1]

        while size >= 1024.0 and len(possible_units) > 0:
            size /= 1024.0
            unit = possible_units.pop()

        return format_str.format(size=size, unit=unit)

    @dataclass
    class PrintState:
        bytes_downloaded: int = 0
        download_size: int = 0
        print_interval: float = (1 / 3) if overwrite_output else 1.0
        last_print_time: float = 0.0
        longest_line: int = 0
        last_bytes_downloaded: int = 0
        done: bool = False

    def print_download_progress(state: PrintState) -> None:
        # Always a no-op if we're not printing anything.
        if not any((show_bar, show_percent, show_bytes_progress)):
            return

        # Avoid printing "100%" multiple times.
        if state.done:
            return
        is_last_iteration = state.bytes_downloaded >= state.download_size
        state.done = is_last_iteration

        curr_time = time.time()

        # Don't immediately print if it'll be retained.
        if not overwrite_output and state.last_print_time == 0.0:
            state.last_print_time = curr_time
            return

        # Only print every so often for performance. The exception is for the
        # last iteration (always print "100%" at least once).
        if (
            curr_time - state.last_print_time < state.print_interval
            and not is_last_iteration
        ):
            return
        state.last_print_time = curr_time

        # If each line we print is retained, print less and less often for slow
        # downloads (that make less than 5% progress between iterations).
        if (
            not overwrite_output
            and state.last_bytes_downloaded != 0
            and (
                (state.bytes_downloaded - state.last_bytes_downloaded)
                / state.download_size
                < 0.05
            )
        ):
            # 20 second max
            state.print_interval = min(state.print_interval * 1.5, 20.0)

        state.last_bytes_downloaded = state.bytes_downloaded

        print_str_parts: list[str] = []

        if show_bar:
            fill_count = round(
                state.bytes_downloaded / state.download_size * bar_inner_width
            )

            bar_color, bar_color_reset = (
                ("".join(bar_inner_style), log.Color.RESET)
                if bar_inner_style is not None
                else ("", "")
            )

            print_str_parts.append(
                (bar_prefix + bar_color)
                + (bar_inner_filled * fill_count)
                + (bar_inner_empty * (bar_inner_width - fill_count))
                + (bar_color_reset + bar_suffix)
            )

        if show_percent:
            percent_done = state.bytes_downloaded / state.download_size * 100
            print_str_parts.append(f"{int(percent_done)}%")

        if show_bytes_progress:
            frac_str = (
                f"{size_with_unit(state.bytes_downloaded)} / "
                + size_with_unit(state.download_size)
            )

            # Put in parenthesis if we're also showing the precent.
            if show_percent:
                frac_str = f"({frac_str})"

            print_str_parts.append(frac_str)

        print_str = " ".join(print_str_parts)

        # Skip the I/O if we don't have anything to print or cover up.
        if len(print_str) == 0 and (
            not overwrite_output or state.longest_line == 0
        ):
            return

        if overwrite_output:
            cleanup_str = " " * (state.longest_line - len(print_str))
            print(f"{print_str}{cleanup_str}\r", end="", flush=True)
        else:
            print(print_str)

        state.longest_line = max(len(print_str), state.longest_line)
        return

    state = PrintState()

    def reporthook(_: int, read_size: int, total_size: int) -> None:
        nonlocal state
        state.bytes_downloaded += read_size
        state.download_size = total_size
        print_download_progress(state)

    log.info(f"Downloading to `{dest_path}`. This may take a while...")
    start_time = time.time()

    if overwrite_output:
        print("\033[?25l", end="")  # hide cursor
        restore_cursor = "\033[?25h"
    else:
        restore_cursor = ""

    try:
        urllib.request.urlretrieve(url, dest_path, reporthook)

    except KeyboardInterrupt:
        print(restore_cursor)

        sh.rm_path(dest_path, allow_missing=True)

        log.warning("Download canceled.")
        if require_download_completes:
            if non_fatal:
                raise DownloadCanceledException()
            log.fatal("The download is required to continue.")
        return False

    except Exception as e:
        print(restore_cursor)

        err_msg = f"Download failed: {e}"
        if non_fatal:
            log.error(err_msg)
            raise
        log.fatal(err_msg)

    print(restore_cursor, end="")

    if overwrite_output and state.longest_line > 0:
        print(f"{' ' * state.longest_line}\r", end="")

    elapsed_time = time.time() - start_time
    log.info(
        f"Download completed in {log.fmt_time(elapsed_time)} "
        + f"({size_with_unit(state.download_size)})."
    )

    return True
