from tqdm import tqdm
import pickle
import git
import argparse


parser = argparse.ArgumentParser()
parser.add_argument(
    "commit_list", help="the commit list on which the merge tools are compared"
)
parser.add_argument(
    "-r",
    "--recompute",
    action="append",
    metavar="tool",
    help="recompute results for a tool even if previously calculated",
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

branch = pickle.load(open(args.commit_list, "rb"))
repo = git.Repo("linux")

for commit in tqdm(branch.backported_commits):
    if args.recompute:
        for file in commit.conflicting_files:
            for tool in args.recompute:
                del file.merge_results[tool]
    commit.update_conflicting_files(repo, args.timeout)

pickle.dump(branch, open(args.commit_list, "wb"))
