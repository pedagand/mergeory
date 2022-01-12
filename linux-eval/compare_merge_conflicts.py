import pickle
from backport_conflicts import SuccessfulMerge, FailedMerge, MERGE_TOOLS
import numpy as np
from tabulate import tabulate
import argparse


parser = argparse.ArgumentParser()
parser.add_argument(
    "commit_list", help="the commit list on which the merge tools are compared"
)
parser.add_argument(
    "-f",
    "--full-commit",
    action="store_true",
    help="display figures for full commit conflict resolution and not individual files",
)
parser.add_argument(
    "-s", "--style", default="grid", help="set the tabulate style for the output"
)
parser.add_argument(
    "-r", "--ratio", action="store_true", help="show ratio instead of cardinals"
)
group = parser.add_mutually_exclusive_group()
group.add_argument(
    "-m",
    "--only-mergeability",
    action="store_true",
    help="only compare mergeability and ignore similarity with upstream",
)
group.add_argument(
    "-b",
    "--only-backported-equiv",
    action="store_true",
    help="only compare equivalence with manually backported patch",
)
args = parser.parse_args()


def result_score(merge_res):
    if merge_res == SuccessfulMerge(True):
        return 2
    elif isinstance(merge_res, SuccessfulMerge):
        return 1
    return 0


def commit_score(pr, merge_tool):
    score = 2
    for file in pr.conflicting_files:
        file_score = result_score(file.merge_results[merge_tool])
        score = min(score, file_score)
    return score


def compare_commit_mergeability(commit, merge_tool1, merge_tool2):
    table = np.zeros((3, 3), dtype=int)
    for file in commit.conflicting_files:
        res1 = file.merge_results[merge_tool1]
        res2 = file.merge_results[merge_tool2]
        table[result_score(res1)][result_score(res2)] += 1
    return table


def compare_merge_conflicts(commit_list, merge_tool1, merge_tool2):
    table = np.zeros((3, 3), dtype=int)
    for commit in commit_list:
        if args.full_commit:
            if commit.only_c_conflicts:
                res1 = commit_score(commit, merge_tool1)
                res2 = commit_score(commit, merge_tool2)
                table[res1][res2] += 1
        else:
            table += compare_commit_mergeability(commit, merge_tool1, merge_tool2)
    return table


branch = pickle.load(open(args.commit_list, "rb"))

for tool1 in MERGE_TOOLS.keys():
    for tool2 in MERGE_TOOLS.keys():
        if tool1 >= tool2:
            continue
        try:
            table = compare_merge_conflicts(branch.backported_commits, tool1, tool2)
        except KeyError:
            continue

        result_names = [" failure", " different", " same"]
        if args.only_mergeability:
            result_names = [" failure", " success"]
            table[1] += table[2]
            table = np.delete(table, 2, 0)
            table[:, 1] += table[:, 2]
            table = np.delete(table, 2, 1)
        elif args.only_backported_equiv:
            result_names = [" different", " same"]
            table[0] += table[1]
            table = np.delete(table, 1, 0)
            table[:, 0] += table[:, 1]
            table = np.delete(table, 1, 1)

        header = ""
        floatfmt = "g"
        if args.ratio:
            count = sum(sum(table))
            header = "N = {}".format(count)
            table = table * 100 / count
            floatfmt = ".2f"

        pretty_table = [
            [tool1 + result_names[i]] + list(table[i]) for i in range(len(result_names))
        ]
        headers = [header] + [tool2 + res_name for res_name in result_names]
        print(
            tabulate(
                pretty_table,
                tablefmt=args.style,
                floatfmt=floatfmt,
                headers=headers,
            )
        )
        print()
