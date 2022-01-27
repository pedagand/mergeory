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


def remove_indent_in_file(in_filename):
    (filename_root, filename_ext) = os.path.splitext(in_filename)
    out_filename = filename_root + "_noindent" + filename_ext
    with open(in_filename, "rb") as infile, open(out_filename, "wb") as outfile:
        for line in infile.readlines():
            outfile.write(line.strip() + b"\n")
    return out_filename


def without_indent(merger):
    def remove_indent_and_merge(base, pr, master, out=None, timeout=None):
        noindent_base = remove_indent_in_file(base)
        noindent_pr = remove_indent_in_file(pr)
        noindent_master = remove_indent_in_file(master)
        exit_code = merger(
            noindent_base,
            noindent_pr,
            noindent_master,
            out=out,
            timeout=timeout,
        )
        os.remove(noindent_base)
        os.remove(noindent_pr)
        os.remove(noindent_master)
        return exit_code

    return remove_indent_and_merge
