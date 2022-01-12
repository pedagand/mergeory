from git import Repo
from backport_conflicts import ReleaseBranch
from tqdm import tqdm
import pickle
import argparse

parser = argparse.ArgumentParser()
parser.add_argument(
    "linux_version", help="the Linux version to extract backported commits from"
)
parser.add_argument(
    "-t",
    "--timeout",
    type=int,
    default=60,
    help="the maximum time in seconds that a tool can take to merge one file before beeing stopped (default: 1 minute)",
)
parser.add_argument(
    "-T",
    "--no-timeout",
    action="store_const",
    const=None,
    dest="timeout",
    help="unset the timeout option to never stop the tools during file merge",
)
args = parser.parse_args()

linux_repo = Repo("linux")
linux_branch = ReleaseBranch(
    "stable/linux-{}.y".format(args.linux_version), "v{}".format(args.linux_version)
)
linux_branch.find_backported_commits(linux_repo)

for commit in tqdm(linux_branch.backported_commits):
    commit.update_conflicting_files(linux_repo, args.timeout)

pickle.dump(linux_branch, open("commits/v{}".format(args.linux_version), "wb"))
