from merge_tools import *

MERGE_TOOLS = {
    "git": merge_file_with_git,
    "patch": patch_merger([], []),
    "syndiff": syndiff_merger(["../syndiff/target/release/syndiff"]),
    "no-elisions": syndiff_merger(
        ["../syndiff/target/release/syndiff", "--no-elisions"]
    ),
}
