import pickle
from backport_conflicts import SuccessfulMerge, FailedMerge, MERGE_TOOLS
import numpy as np
from tabulate import tabulate
import argparse
from enum import IntEnum


parser = argparse.ArgumentParser()
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
parser.add_argument(
    "-p",
    "--only-parsed",
    action="store_true",
    help="filter out files that do not parse or commits that contain a non parsable file",
)
parser.add_argument(
    "-t",
    "--include-trivial",
    action="store_true",
    help="include the files that all the tools can merge identically to upstream",
)
parser.add_argument(
    "-w",
    "--ignore-whitespace",
    action="store_true",
    help="ignore differences that occur in whitespaces only",
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
subparser = parser.add_subparsers(
    description="choose how to show the results", metavar="subcommand", required=True
)
compare_parser = subparser.add_parser("compare", help="compare tool results two by two")
compare_parser.set_defaults(action="compare")
compare_parser.add_argument(
    "branch", help="the branch on which the merge tools are compared"
)
compare_parser.add_argument(
    "tool", nargs="?", help="show only comparison tables against this tool"
)
compare_parser.add_argument(
    "other_tool",
    nargs="?",
    help="show only the comparison table between the two provided tools",
)
list_parser = subparser.add_parser(
    "list", help="displays a list of individual tool results"
)
list_parser.set_defaults(action="list")
list_parser.add_argument(
    "branch", help="the branch on which the merge tools are compared"
)
history_parser = subparser.add_parser(
    "history", help="show a single tool results across branches"
)
history_parser.set_defaults(action="history")
history_parser.add_argument("tool", help="the tool that will be shown")
history_parser.add_argument("branches", nargs="+", help="the set of branches to show")
args = parser.parse_args()


def is_identical(merge_res):
    if args.ignore_whitespace:
        return merge_res.same_without_space
    else:
        return merge_res.same_as_backported_file


class Score(IntEnum):
    FAILURE = 0
    DIFFERENT = 1
    IDENTICAL = 2
    TRIVIAL = 3


def result_score(merge_results, merge_tool):
    merge_res = merge_results[merge_tool]
    if isinstance(merge_res, SuccessfulMerge):
        if is_identical(merge_res):
            if args.include_trivial:
                return Score.IDENTICAL
            else:
                for other_res in merge_results.values():
                    if not isinstance(other_res, SuccessfulMerge) or not is_identical(
                        other_res
                    ):
                        return Score.IDENTICAL
                return Score.TRIVIAL
        else:
            return Score.DIFFERENT
    else:
        return Score.FAILURE


def commit_score(pr, merge_tool):
    score = Score.TRIVIAL
    for file in pr.conflicting_files:
        file_score = result_score(file.merge_results, merge_tool)
        score = min(score, file_score)
    return score


def compare_commit_mergeability(commit, merge_tool1, merge_tool2):
    table = np.zeros((3, 3), dtype=int)
    for file in commit.conflicting_files:
        if not args.only_parsed or file.can_parse:
            res1 = result_score(file.merge_results, merge_tool1)
            if res1 == Score.TRIVIAL:
                continue
            res2 = result_score(file.merge_results, merge_tool2)
            table[res1][res2] += 1
    return table


def compare_merge_conflicts(commit_list, merge_tool1, merge_tool2):
    table = np.zeros((3, 3), dtype=int)
    for commit in commit_list:
        if args.full_commit:
            if commit.only_c_conflicts and (
                not args.only_parsed or commit.can_parse_conflicts
            ):
                res1 = commit_score(commit, merge_tool1)
                if res1 == Score.TRIVIAL:
                    continue
                res2 = commit_score(commit, merge_tool2)
                table[res1][res2] += 1
        else:
            table += compare_commit_mergeability(commit, merge_tool1, merge_tool2)
    return table


def tool_score(commit_list, merge_tool):
    table = [0, 0, 0]
    for commit in commit_list:
        if args.full_commit:
            if commit.only_c_conflicts and (
                not args.only_parsed or commit.can_parse_conflicts
            ):
                res = commit_score(commit, merge_tool)
                if res == Score.TRIVIAL:
                    continue
                table[res] += 1
        else:
            for file in commit.conflicting_files:
                if not args.only_parsed or file.can_parse:
                    res = result_score(file.merge_results, merge_tool)
                    if res == Score.TRIVIAL:
                        continue
                    table[res] += 1
    return table


def get_column_names():
    if args.only_mergeability:
        return ["failure", "success"]
    elif args.only_backported_equiv:
        return ["different", "same"]
    else:
        return ["failure", "different", "same"]


def make_table(headers, data):
    floatfmt = ".2f" if args.ratio else "g"
    return tabulate(data, headers=headers, tablefmt=args.style, floatfmt=floatfmt)


def compare_tools(branch, tool1, tool2):
    table = compare_merge_conflicts(branch.backported_commits, tool1, tool2)

    if args.only_mergeability:
        table[Score.IDENTICAL] += table[Score.DIFFERENT]
        table = np.delete(table, Score.DIFFERENT, 0)
        table[:, Score.IDENTICAL] += table[:, Score.DIFFERENT]
        table = np.delete(table, Score.DIFFERENT, 1)
    elif args.only_backported_equiv:
        table[Score.FAILURE] += table[Score.DIFFERENT]
        table = np.delete(table, Score.DIFFERENT, 0)
        table[:, Score.FAILURE] += table[:, Score.DIFFERENT]
        table = np.delete(table, Score.DIFFERENT, 1)

    column_names = get_column_names()
    header = ""
    if args.ratio:
        count = sum(sum(table))
        header = "N = {}".format(count)
        table = table * 100 / count

    pretty_table = [
        [tool1 + " " + column_names[i]] + list(table[i]) for i in range(len(table))
    ]
    headers = [header] + [tool2 + " " + col_name for col_name in column_names]
    print(make_table(headers, pretty_table))


def normalize_results(score):
    if args.only_mergeability:
        score = [
            score[Score.FAILURE],
            score[Score.DIFFERENT] + score[Score.IDENTICAL],
        ]
    elif args.only_backported_equiv:
        score = [
            score[Score.FAILURE] + score[Score.DIFFERENT],
            score[Score.IDENTICAL],
        ]

    if args.ratio:
        count = sum(score)
        score = [s * 100 / count for s in score]

    return score


def list_results(branch):
    res_list = []
    for tool_name in MERGE_TOOLS.keys():
        try:
            score = tool_score(branch.backported_commits, tool_name)
        except KeyError:
            continue

        res_list.append([tool_name] + normalize_results(score))

    headers = ["tool"] + get_column_names()
    print(make_table(headers, res_list))


def show_history():
    res_list = []
    for branch_name in args.branches:
        branch = pickle.load(open(branch_name, "rb"))
        score = tool_score(branch.backported_commits, args.tool)
        res_list.append([branch.branch] + normalize_results(score))

    headers = ["branch"] + get_column_names()
    print(make_table(headers, res_list))


if args.action == "compare":
    branch = pickle.load(open(args.branch, "rb"))
    if args.tool:
        if args.other_tool:
            compare_tools(branch, args.tool, args.other_tool)
        else:
            for tool2 in MERGE_TOOLS.keys():
                if args.tool == tool2:
                    continue
                try:
                    compare_tools(branch, args.tool, tool2)
                    print()
                except KeyError as err:
                    if err.args[0] == args.tool:
                        raise
                    continue
    else:
        for tool1 in MERGE_TOOLS.keys():
            for tool2 in MERGE_TOOLS.keys():
                if tool1 >= tool2:
                    continue
                try:
                    compare_tools(branch, tool1, tool2)
                    print()
                except KeyError:
                    continue
elif args.action == "list":
    branch = pickle.load(open(args.branch, "rb"))
    list_results(branch)
elif args.action == "history":
    show_history()
