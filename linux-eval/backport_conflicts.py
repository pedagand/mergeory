from git import Repo, IndexFile, Blob, GitCommandError
from merge_tools import TimeoutExpired
from merge_tool_list import MERGE_TOOLS
import os
import filecmp
from dataclasses import dataclass


class ReleaseBranch:
    def __init__(self, branch, initial_release):
        self.branch = branch
        self.initial_release = initial_release

    def find_backported_commits(self, repo):
        if not hasattr(self, "backported_commits"):
            self.backported_commits = []
            initial_commit = repo.commit(self.initial_release)
            for commit in repo.iter_commits(self.branch):
                if commit == initial_commit:
                    return
                message_lines = commit.message.splitlines()
                if len(message_lines) < 3:
                    continue
                upstream_line = message_lines[2].split()
                if (
                    len(upstream_line) == 3
                    and upstream_line[0] == "commit"
                    and upstream_line[2] == "upstream."
                ):
                    try:
                        upstream_commit = repo.commit(upstream_line[1])
                    except:
                        print(
                            "Upstream commit could not be found for {}".format(
                                commit.hexsha
                            )
                        )
                        continue
                    self.backported_commits.append(
                        BackportedCommit(commit.hexsha, upstream_commit.hexsha)
                    )

    def __repr__(self):
        return "ReleaseBranch({})".format(self.branch)


def is_c_file_conflict(filename, blobs):
    if not (filename.endswith(".c") or filename.endswith(".h")):
        return False
    if len(blobs) != 3:
        return False  # file-level conflict
    return True


class BackportedCommit:
    def __init__(self, release_commit, upstream_commit):
        self.release_commit = release_commit
        self.upstream_commit = upstream_commit

    def __repr__(self):
        return "BackportedCommit({} from {})".format(
            self.release_commit, self.upstream_commit
        )

    def try_cherry_pick_upstream(self, repo):
        if not hasattr(self, "can_cherry_pick_upstream"):
            repo.head.reference = repo.commit(self.release_commit + "^")
            repo.head.reset(working_tree=True)
            try:
                repo.git.cherry_pick(self.upstream_commit)
                self.can_cherry_pick_upstream = True
                self.same_as_cherry_pick = (
                    len(repo.head.commit.diff(self.release_commit)) == 0
                )
            except GitCommandError:
                self.can_cherry_pick_upstream = False

    def update_conflicting_files(self, repo, timeout=None):
        if not hasattr(self, "conflicting_files"):
            merge_index = IndexFile.from_tree(
                repo,
                self.upstream_commit + "^",
                self.upstream_commit,
                self.release_commit + "^",
            )
            self.conflicting_files = []
            self.only_c_conflicts = True
            for (filename, blobs) in merge_index.unmerged_blobs().items():
                if is_c_file_conflict(filename, blobs):
                    self.conflicting_files.append(
                        ConflictingFile(self.release_commit, filename, blobs)
                    )
                else:
                    self.only_c_conflicts = False

        for conflict_file in self.conflicting_files:
            conflict_file.compute_merge(repo, timeout)


@dataclass
class SuccessfulMerge:
    same_as_backported_file: bool


@dataclass
class FailedMerge:
    exit_code: int


@dataclass
class TimedoutMerge:
    elapsed_time: int


class ConflictingFile:
    def __init__(self, commit_number, filename, blobs):
        self.merge_results = dict()
        self.commit_number = commit_number
        self.filename = filename
        for (stage, blob) in blobs:
            if stage == 1:
                self.dev_base = blob.binsha
            if stage == 2:
                self.modif = blob.binsha
            if stage == 3:
                self.release_base = blob.binsha

    def __repr__(self):
        return "ConflictingFile({}, {})".format(self.filename, self.commit_number)

    def write_files(self, repo, folder):
        (base_filename, ext) = os.path.splitext(os.path.basename(self.filename))
        dev_base_filename = os.path.join(folder, base_filename + ".dev_base" + ext)
        modif_filename = os.path.join(folder, base_filename + ".modif" + ext)
        release_base_filename = os.path.join(
            folder, base_filename + ".release_base" + ext
        )
        Blob(repo, self.dev_base).stream_data(open(dev_base_filename, "wb"))
        Blob(repo, self.modif).stream_data(open(modif_filename, "wb"))
        Blob(repo, self.release_base).stream_data(open(release_base_filename, "wb"))
        return (dev_base_filename, modif_filename, release_base_filename)

    def write_backported_file(self, repo, folder):
        release_tree = repo.tree(self.commit_number)
        (base_filename, ext) = os.path.splitext(os.path.basename(self.filename))
        backported_filename = os.path.join(folder, base_filename + ".backported" + ext)
        release_tree[self.filename].stream_data(open(backported_filename, "wb"))
        return backported_filename

    def need_recomputation(self, tool, timeout=None):
        if not tool in self.merge_results:
            return True
        if isinstance(self.merge_results[tool], TimedoutMerge):
            if timeout is None:
                return True
            if timeout > self.merge_results[tool].timeout:
                return True
        return False

    def compute_merge(self, repo, timeout=None):
        files = self.write_files(repo, "/tmp")
        backported_file = self.write_backported_file(repo, "/tmp")
        for (tool, merge_fn) in MERGE_TOOLS.items():
            if self.need_recomputation(tool, timeout):
                merged_filename = "/tmp/merged"
                try:
                    merge_exit_code = merge_fn(
                        *files, out=open(merged_filename, "w"), timeout=timeout
                    )
                    if merge_exit_code == 0:
                        self.merge_results[tool] = SuccessfulMerge(
                            filecmp.cmp(backported_file, merged_filename, shallow=False)
                        )
                    else:
                        self.merge_results[tool] = FailedMerge(merge_exit_code)
                except TimeoutExpired:
                    self.merge_results[tool] = TimedoutMerge(timeout)
                os.remove(merged_filename)
        for file in files:
            os.remove(file)
        os.remove(backported_file)
