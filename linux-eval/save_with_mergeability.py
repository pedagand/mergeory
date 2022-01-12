import pickle
from backport_conflicts import SuccessfulMerge, FailedMerge, TimedoutMerge, MERGE_TOOLS
import git
from os import makedirs
import argparse

parser = argparse.ArgumentParser()
parser.add_argument(
    "commit_list", help="the commit list from which matching files are extracted"
)
parser.add_argument("dest_folder", help="a destination folder for the matched files")
parser.add_argument(
    "-m",
    "--mergeable-by",
    action="append",
    default=[],
    metavar="tool",
    help="take only files that can be merged by the tool",
)
parser.add_argument(
    "-f",
    "--fail-merge-by",
    action="append",
    default=[],
    metavar="tool",
    help="take only files that could not be merged by the tool",
)
parser.add_argument(
    "-e",
    "--fail-with-error",
    action="append",
    nargs=2,
    default=[],
    metavar=("tool", "code"),
    help="take only files where the tool exited with the given error code",
)
parser.add_argument(
    "-t",
    "--merge-timeout-with",
    action="append",
    default=[],
    metavar="tool",
    help="take only files for which merge exceeded timeout with the tool",
)
parser.add_argument(
    "-s",
    "--same-as-backported",
    action="append",
    default=[],
    metavar="tool",
    help="take only files where the merged output is the same as the version backported in the release",
)
parser.add_argument(
    "-d",
    "--different-than-backported",
    action="append",
    default=[],
    metavar="tool",
    help="take only files where the merged output is different than the backported version",
)
args = parser.parse_args()


def results_match_constraints(results):
    for tool in args.mergeable_by:
        if not isinstance(results[tool], SuccessfulMerge):
            return False
    for tool in args.fail_merge_by:
        if not isinstance(results[tool], FailedMerge):
            return False
    for tool in args.merge_timeout_with:
        if not isinstance(results[tool], TimedoutMerge):
            return False
    for (tool, code) in args.fail_with_error:
        if results[tool] != FailedMerge(code):
            return False
    for tool in args.same_as_backported:
        if results[tool] != SuccessfulMerge(True):
            return False
    for tool in args.different_than_backported:
        if results[tool] == SuccessfulMerge(True):
            return False
    return True


branch = pickle.load(open(args.commit_list, "rb"))
repo = git.Repo("linux")

for commit in branch.backported_commits:
    for conflict_file in commit.conflicting_files:
        if not results_match_constraints(conflict_file.merge_results):
            continue

        commit_folder = "{}/{}".format(args.dest_folder, commit.release_commit)
        makedirs(commit_folder, exist_ok=True)
        conflict_file.write_files(repo, commit_folder)
        conflict_file.write_backported_file(repo, commit_folder)
