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
    "-l",
    "--list",
    action="store_true",
    help="display results as a flat list of tool pairs",
)
compare_parser.add_argument(
    "-R",
    "--one-fail-ratio",
    action="store_true",
    help="show ratio among tests that fail on at least one of the two compared tools",
)
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
history_parser.add_argument(
    "tool",
    help="the tool that will be shown, if set to 'trivial' counts the number of trivial commits instead",
)
history_parser.add_argument("branches", nargs="*", help="the set of branches to show")
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


def is_trivial(merge_results):
    for res in merge_results.values():
        if not isinstance(res, SuccessfulMerge) or not is_identical(res):
            return False
    return True


def result_score(merge_results, merge_tool):
    merge_res = merge_results[merge_tool]
    if isinstance(merge_res, SuccessfulMerge):
        if is_identical(merge_res):
            if args.include_trivial:
                return Score.IDENTICAL
            else:
                if is_trivial(merge_results):
                    return Score.TRIVIAL
                else:
                    return Score.IDENTICAL
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


def normalize_comparison_table(table):
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
    if args.ratio:
        count = sum(sum(table))
        if args.one_fail_ratio:
            count -= table[-1][-1]
        table *= 100 / count


def show_one_tool_comparison_table(branch, tool1, tool2):
    table = compare_merge_conflicts(branch.backported_commits, tool1, tool2)
    normalize_comparison_table(table)

    column_names = get_column_names()
    pretty_table = [
        [tool1 + " " + column_names[i]] + list(table[i]) for i in range(len(table))
    ]
    headers = [""] + [tool2 + " " + col_name for col_name in column_names]

    print(make_table(headers, pretty_table))


def show_tool_comparison_tables(branch):
    if args.tool:
        if args.other_tool:
            show_one_tool_comparison_table(branch, args.tool, args.other_tool)
        else:
            for tool2 in MERGE_TOOLS.keys():
                if args.tool == tool2:
                    continue
                try:
                    show_one_tool_comparison_table(branch, args.tool, tool2)
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
                    show_one_tool_comparison_table(branch, tool1, tool2)
                    print()
                except KeyError:
                    continue


def tool_comparison_entry(branch, tool1, tool2):
    table = compare_merge_conflicts(branch.backported_commits, tool1, tool2)
    normalize_comparison_table(table)
    return [tool1, tool2] + list(table.flatten())


def show_tool_comparison_list(branch):
    res_list = []
    if args.tool:
        if args.other_tool:
            res_list.append(tool_comparison_entry(branch, args.tool, args.other_tool))
        else:
            for tool2 in MERGE_TOOLS.keys():
                try:
                    res_list.append(tool_comparison_entry(branch, args.tool, tool2))
                except KeyError as err:
                    if err.args[0] == args.tool:
                        raise
    else:
        for tool1 in MERGE_TOOLS.keys():
            for tool2 in MERGE_TOOLS.keys():
                if tool1 > tool2:
                    continue
                try:
                    res_list.append(tool_comparison_entry(branch, tool1, tool2))
                except KeyError:
                    pass

    single_column_names = get_column_names()
    headers = ["tool1", "tool2"] + [
        res1 + "-" + res2
        for res1 in single_column_names
        for res2 in single_column_names
    ]
    print(make_table(headers, res_list))


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


def show_result_list(branch):
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


def show_trivial_history():
    res_list = []
    for branch_name in args.branches:
        branch = pickle.load(open(branch_name, "rb"))
        counts = [0, 0]
        for commit in branch.backported_commits:
            if args.full_commit:
                if commit.only_c_conflicts and (
                    not args.only_parsed or commit.can_parse_conflicts
                ):
                    trivial = True
                    for file in commit.conflicting_files:
                        if not is_trivial(file.merge_results):
                            trivial = False
                    counts[int(trivial)] += 1
            else:
                for file in commit.conflicting_files:
                    if not args.only_parsed or file.can_parse:
                        counts[int(is_trivial(file.merge_results))] += 1
        res_list.append([branch.branch] + counts)

    headers = ["branch", "interesting", "trivial"]
    print(make_table(headers, res_list))


if args.action == "compare":
    branch = pickle.load(open(args.branch, "rb"))
    if args.one_fail_ratio:
        args.ratio = True
    if args.list:
        show_tool_comparison_list(branch)
    else:
        show_tool_comparison_tables(branch)
elif args.action == "list":
    branch = pickle.load(open(args.branch, "rb"))
    show_result_list(branch)
elif args.action == "history":
    if args.tool == "trivial":
        show_trivial_history()
    else:
        show_history()
