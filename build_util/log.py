"""
Contains logging functions.
"""

import sys
import os
from enum import Enum
from typing import NoReturn, Any, Optional, Union

_color_enabled = bool(sys.stdout.isatty())


class Color(str, Enum):
    @staticmethod
    def enabled() -> bool:
        return _color_enabled

    RESET = "\033[0m" if enabled() else ""
    NONE = ""

    BLACK = "\033[30m" if enabled() else ""
    RED = "\033[31m" if enabled() else ""
    GREEN = "\033[32m" if enabled() else ""
    YELLOW = "\033[33m" if enabled() else ""
    BLUE = "\033[34m" if enabled() else ""
    MAGENTA = "\033[35m" if enabled() else ""
    CYAN = "\033[36m" if enabled() else ""
    WHITE = "\033[37m" if enabled() else ""
    DEFAULT_COLOR = "\033[39m" if enabled() else ""

    BOLD = "\033[1m" if enabled() else ""
    FAINT = "\033[2m" if enabled() else ""
    NORMAL_INTENSITY = "\033[22m" if enabled() else ""

    ERROR = RED if enabled() else ""
    WARNING = YELLOW if enabled() else ""
    INFO = CYAN if enabled() else ""
    SUCCESS = GREEN if enabled() else ""
    CONFIRM = MAGENTA if enabled() else ""
    ACTION_NEEDED = MAGENTA + BOLD if enabled() else ""
    COMMAND = BLUE if enabled() else ""

    def __str__(self):
        return str.__str__(self)


def fatal(
    *args: Any,
    include_run_again_msg: bool = True,
    sep: Optional[str] = " ",
) -> NoReturn:
    """
    Print an error and exit.
    """

    sys.stdout.flush()

    print(f"{Color.ERROR}FATAL{Color.RESET}: ", end="", file=sys.stderr)
    print(*args, sep=sep, file=sys.stderr)
    if include_run_again_msg:
        print(
            "\nPlease run this script again once the issue is resolved.",
            file=sys.stderr,
            flush=True,
        )

    os._exit(1)


def warning(
    *args: Any,
    sep: Optional[str] = " ",
    end: Optional[str] = "\n",
    flush: bool = False,
) -> None:
    """
    Print a warning.
    """

    print(
        f"{Color.WARNING}WARNING{Color.RESET}: ",
        end="",
        file=sys.stderr,
        flush=False,
    )
    print(*args, sep=sep, file=sys.stderr, end=end, flush=flush)


def error(
    *args: Any,
    sep: Optional[str] = " ",
    end: Optional[str] = "\n",
    flush: bool = False,
) -> None:
    """
    Print an error.
    """

    print(
        f"{Color.ERROR}ERROR{Color.RESET}: ",
        end="",
        file=sys.stderr,
        flush=False,
    )
    print(*args, sep=sep, file=sys.stderr, end=end, flush=flush)


def info(
    *args: Any,
    sep: Optional[str] = " ",
    end: Optional[str] = "\n",
    flush: bool = False,
) -> None:
    """
    Print some info.
    """

    print(f"{Color.INFO}INFO{Color.RESET}: ", end="", flush=False)
    print(*args, sep=sep, end=end, flush=flush)


def success(
    *args: Any,
    sep: Optional[str] = " ",
    end: Optional[str] = "\n",
    flush: bool = False,
) -> None:
    """
    Print that the process is done (success).
    """

    print(f"{Color.SUCCESS}SUCCESS{Color.RESET}: ", end="", flush=False)
    print(*args, sep=sep, end=end, flush=flush)


def fmt_time(secs: float) -> str:
    """
    Formats a time to be human readable (e.g. `"1 minute and 15 seconds"`).
    """

    hours, sub_hour_secs = divmod(int(secs), 3600)
    mins, secs = sub_hour_secs // 60, (sub_hour_secs % 60) + (secs - int(secs))

    def pluralize(noun: str, n: Union[int, float]) -> str:
        return f"{noun}{'s' if n < 0.95 or n >= 1.05 else ''}"

    hours_str = f"{hours} {pluralize('hour', hours)}"
    mins_str = f"{mins} {pluralize('min', mins)}"
    secs_str = (
        f"{int(secs)} " if round(secs, 1).is_integer() else f"{secs:.1f} "
    ) + pluralize("second", secs)

    if hours:
        if mins:
            return f"{hours_str}, {mins_str}, and {secs_str}"
        return f"{hours_str} and {secs_str}"
    if mins:
        return f"{mins_str} and {secs_str}"
    return secs_str
