import subprocess
import os

from subprocess import TimeoutExpired


def merge_file_with_git(base, pr, master, out=None, timeout=None):
    if out is None:
        out = subprocess.DEVNULL
    return subprocess.call(
        ["git", "merge-file", "-p", pr, base, master], stdout=out, timeout=timeout
    )


def compute_and_apply_patch(base, pr, master, out=None, timeout=None):
    if out is None:
        out = subprocess.DEVNULL
    with subprocess.Popen(["diff", "-p", base, pr], stdout=subprocess.PIPE) as diff:
        exit_value = subprocess.call(
            ["patch", "--force", "-s", "-r", "-", "-o", "-", master],
            stdin=diff.stdout,
            stderr=subprocess.DEVNULL,
            stdout=out,
            timeout=timeout,
        )
    return exit_value


def syndiff_merger(cmd):
    def merge_file_with_syndiff(base, pr, master, out=None, timeout=None):
        quiet = out is None
        if out is None:
            out = subprocess.DEVNULL
        return subprocess.call(
            cmd
            + [
                "-q" if quiet else "-m",
                base,
                pr,
                master,
            ],
            stdout=out,
            timeout=timeout,
        )

    return merge_file_with_syndiff
