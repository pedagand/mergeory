from merge_tools import *

MERGE_TOOLS = {
    "git": merge_file_with_git,
    "patch": compute_and_apply_patch,
    "syndiff": syndiff_merger(["../syndiff/target/release/syndiff"]),
    "no-elisions": syndiff_merger(
        ["../syndiff/target/release/syndiff", "--no-elisions"]
    ),
}
