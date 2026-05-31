"""
Contains info about the build of FFmpeg being used.
"""

import os
import shutil
import tarfile
from typing import Literal, Optional

import build_util.sh as sh
import build_util.log as log
import build_util.user as user

FFMPEG_VERSION = "8.1"
"""
The version of FFmpeg being used.
"""

FFMPEG_URL = "https://git.ffmpeg.org/gitweb/ffmpeg.git/tag/a65b3bfe9dacc3b20597ef199d0afdd8bc8128e2"
"""
The URL of the Git tag of the version of FFmpeg being used.
"""


def path(absolute: bool = False) -> str:
    """
    The path to the FFmpeg build directory.
    """

    dir_path = f"{sh.cache_dir(create=False)}{os.sep}ffmpeg"
    if not absolute:
        return dir_path
    try:
        return str(os.path.abspath(dir_path))
    except:
        log.fatal(f"Failed to get absolute path of `{dir_path}`.")


def license_file(absolute: bool = False) -> str:
    """
    The path to the FFmpeg in the FFmpeg build directory.
    """

    return f"{path(absolute=absolute)}{os.sep}COPYING.LGPLv2.1"


def ffmpeg_exe(
    absolute: bool = False, arch: Literal["x86_64", "arm64"] = sh.build_arch()
) -> str:
    """
    The path to the `ffmpeg` binary in the FFmpeg build directory.

    On Linux this will point to the `x86_64` or `arm64` variant depending on the
    host platform by default. Configure this by providing `arch`. Only `x86_64`
    is supported for Windows.
    """

    if sh.build_os() == "windows":
        return f"{path(absolute=absolute)}\\bin\\x64\\ffmpeg.exe"
    elif sh.build_os() == "darwin":  # macOS
        return f"{path(absolute=absolute)}/bin/ffmpeg"
    elif sh.build_os() == "linux":
        arch = "amd64" if arch == "x86_64" else arch
        return f"{path(absolute=absolute)}/bin/{arch}/ffmpeg"


def dylib_folder(
    absolute: bool = False, arch: Literal["x86_64", "arm64"] = sh.build_arch()
) -> str:
    """
    The path to the library folder in the FFmpeg build directory with the
    `.dll`/`.dylib`/`.so` files.

    On Linux this will point to the `x86_64` or `arm64` variant depending on the
    host platform by default. Configure this by providing `arch`. Only `x86_64`
    is supported for Windows.
    """

    if sh.build_os() == "windows":
        return f"{path(absolute=absolute)}\\bin\\x64"
    elif sh.build_os() == "darwin":  # macOS
        return f"{path(absolute=absolute)}/lib"
    elif sh.build_os() == "linux":
        arch = "amd64" if arch == "x86_64" else arch
        return f"{path(absolute=absolute)}/lib/{arch}"


def pkgconfig(
    absolute: bool = False, arch: Literal["x86_64", "arm64"] = sh.build_arch()
) -> str:
    """
    The path to the `pkgconfig` folder in the FFmpeg build directory.

    On Linux this will point to the `x86_64` or `arm64` variant depending on the
    host platform by default. Configure this by providing `arch`. Only `x86_64`
    is supported for Windows.
    """

    if sh.build_os() == "windows":
        return f"{path(absolute=absolute)}\\lib\\x64\\pkgconfig"
    elif sh.build_os() == "darwin":  # macOS
        return f"{path(absolute=absolute)}/lib/pkgconfig"
    elif sh.build_os() == "linux":
        arch = "amd64" if arch == "x86_64" else arch
        return f"{path(absolute=absolute)}/lib/{arch}/pkgconfig"


def ensure_exists_locally(non_fatal: bool = False) -> None:
    """
    Whether FFmpeg is installed locally or not.
    """

    for folder in (
        path(),
        f"{path()}/bin",
        f"{path()}/include",
        f"{path()}/lib",
        dylib_folder(arch="x86_64"),
        dylib_folder(arch="arm64"),
        pkgconfig(arch="x86_64"),
        pkgconfig(arch="arm64"),
    ):
        sh.ensure_path_exists(folder, kind="dir", non_fatal=non_fatal)
    for file in (
        license_file(),
        ffmpeg_exe(arch="x86_64"),
        ffmpeg_exe(arch="arm64"),
    ):
        sh.ensure_path_exists(file, kind="file", non_fatal=non_fatal)


def dylibs(
    arch: Literal["x86_64", "arm64"] = sh.build_arch(),
) -> list[(str, str)]:
    """
    The paths and names of all important `.dll`/`.dylib`/`.so` files in the the
    FFmpeg build directory.

    Duplicates (that simlink to each other) will not be returned (the
    lexicographically last file is used). The resulting paths may be to
    symlinks. All-inclusive FFmpeg files (e.g. `libffmpeg*.dylib`) are filtered
    out.

    On Linux this will point to the `x86_64` or `arm64` variant depending on the
    host platform by default. Configure this by providing `arch`. Only `x86_64`
    is supported for Windows.
    """

    if sh.build_os() == "windows":
        ext = ".dll"
    elif sh.build_os() == "darwin":  # macOS
        ext = ".dylib"
    elif sh.build_os() == "linux":
        ext = ".so"

    try:
        dylib_dir = dylib_folder(arch=arch)
        dylibs = {
            str(os.path.realpath(f"{dylib_dir}/{item}")): item
            for item in os.listdir(dylib_dir)
            if item.endswith(ext)
            and not item.startswith(("libffmpeg", "ffmpeg"))
        }
    except:
        log.fatal("Failed to enumerate FFmpeg dynamic library files.")

    return list(dylibs.items())


def download_url() -> str:
    """
    The URL we're downloading FFmpeg from (platform specific).
    """

    # We're getting our builds from here: https://github.com/wang-bin/avbuild
    # Only 1 maintainer unfortunately, but it has existed for a decent while and
    # it's got a decent amount of downloads too:
    # https://sourceforge.net/projects/avbuild/files/stats/timeline?dates=2017-02-13%20to%202026-05-31&period=monthly
    avbuild = "https://sourceforge.net/projects/avbuild/files"

    if sh.build_os() == "windows":
        return f"{avbuild}/windows-desktop/ffmpeg-8.1-windows-desktop-vs2026ltl-default.7z/download"
    elif sh.build_os() == "darwin":  # macOS
        return f"{avbuild}/macOS/ffmpeg-8.1-macOS-default.tar.xz/download"
    elif sh.build_os() == "linux":
        return f"{avbuild}/linux/ffmpeg-8.1-linux-clang-default.tar.xz/download"


def get_ffmpeg():
    """
    Downloads FFmpeg builds from the internet if it's not installed.
    """

    try:
        ensure_exists_locally(non_fatal=True)
    except sh.DoesntExistException:
        log.info("FFmpeg not found locally.")
    else:
        log.info(f"Found FFmpeg v{FFMPEG_VERSION} installed locally.")
        return

    temp_dir = sh.temp_dir()
    archive_ext = ".7z" if sh.build_os() == "windows" else ".tar.xz"
    archive_path = f"{temp_dir}{os.sep}ffmpeg{archive_ext}"

    user.ask_to_download(download_url(), archive_path)
    log.info("FFmpeg downloaded.")

    extracted_archive_path = f"{temp_dir}{os.sep}ffmpeg"

    if archive_ext == ".tar.xz":
        log.info("Extracting downloaded FFmpeg build archive...")
        try:
            with tarfile.open(archive_path, mode="r:xz") as tar_xz:
                tar_xz.extractall(extracted_archive_path)
        except:
            log.fatal("Failed to extract FFmpeg build archive.")

    else:  # it's a .7z archive :(

        def get_7z_cmd() -> Optional[str]:
            from build_util.platforms import win

            cahce_dir_7zr_path = f"{sh.cache_dir(create=False)}\\7zr.exe"

            log.info("Checking for 7z utility...")
            sevenzip_cmd: Optional[str] = None
            for cmd in (
                cahce_dir_7zr_path,
                "7z",
                "7z.exe",
                f"{win.program_files(x86=True)}\\7-Zip\\7z.exe",
                f"{win.program_files()}\\7-Zip\\7z.exe",
            ):
                try:
                    sh.ensure_cmd_exists(cmd, non_fatal=True)
                except sh.DoesntExistException:
                    continue
                else:
                    sevenzip_cmd = cmd
                    break

            if sevenzip_cmd is not None:
                log.info("7z utility found.")
                return sevenzip_cmd
            else:
                log.info("7z utility not found.")

            if user.ask_to_download(
                "https://github.com/ip7z/7zip/releases/download/26.01/7zr.exe",
                cahce_dir_7zr_path,
                "A `.7z` archive was downloaded and can't be extracted "
                + "automatically without the 7z utility. If you skip this "
                + "download you'll be prompted to extract manually.",
                require_download_completes=False,
            ):
                return cahce_dir_7zr_path
            else:
                return None

        def extract_7z_archive() -> None:
            if (sevenzip_cmd := get_7z_cmd()) is not None:
                log.info("Auto-extracting FFmpeg zip file with 7z utility.")
                try:
                    sh.run_cmd(
                        sevenzip_cmd,
                        "x",
                        archive_path,
                        f"-o{extracted_archive_path}",
                    )
                except:
                    log.warning("Failed to extract with 7z utility.")
                else:
                    return

            # Without the 7z command utility we can't extract it for the user.
            # They need to use the file explorer UI.

            log.info("Attempting to open file explorer on FFmpeg zip file...")
            sh.start_cmd(
                "explorer",
                "/select,",
                f"{os.path.abspath(archive_path)}",
            )
            user.action_needed(
                f"Please extract `{archive_path}` to "
                + f"`{extracted_archive_path}`.",
            )

            try:
                sh.ensure_path_exists(
                    extracted_archive_path, kind="dir", non_fatal=True
                )
            except sh.DoesntExistException:
                log.fatal("The FFmpeg directory wasn't extracted.")

        extract_7z_archive()

    log.info("Extracted FFmpeg build archive.")

    try:
        archive_items = os.listdir(extracted_archive_path)
        if len(archive_items) != 1:
            log.fatal("Unexpected data found in downloaded build archive.")

        sh.cache_dir(create=True)  # Make sure cache directory exists
        shutil.move(f"{extracted_archive_path}/{archive_items[0]}", path())
    except:
        log.fatal("Failed to cache FFmpeg build from downloaded archive.")
    log.info("FFmpeg build cached.")

    sh.rm_path(temp_dir)

    # On Windows we need the contents of `x64` in the root of the `lib` dir so
    # we'll create a bunch of simlinks.
    if sh.build_os() == "windows":
        lib_dir = f"{path()}\\lib"
        try:
            for item in os.listdir(f"{lib_dir}\\x64"):
                os.symlink(
                    src := os.path.abspath(f"{lib_dir}\\x64\\{item}"),
                    os.path.abspath(f"{lib_dir}\\{item}"),
                    target_is_directory=os.path.isdir(src),
                )
        except:
            log.fatal("Failed to clean FFmpeg archive structure.")

    ensure_exists_locally()
    log.info(f"FFmpeg v{FFMPEG_VERSION} installed locally.")
