import merge_tools
import argparse

parser = argparse.ArgumentParser()
parser.add_argument("filename", help="the file to strip indentation from")
args = parser.parse_args()

merge_tools.remove_indent_in_file(args.filename)
