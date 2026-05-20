###==========================================================================###
### args.py - by Drayux                                                      ###
### SCRIPT ARGUMENT PARSING                                                  ###
### Parses and processes arguments passed into script                        ###
###==========================================================================###

from collections import namedtuple
from os import chdir, path
from sys import excepthook, exit

from func.util import log

ArgList = namedtuple('ArgList', ['directory', 'logging', 'stats', 'verbose', 'module'])

def usage():
    print("Usage: wizard_automatus.py [OPTION]... [MODULE]")
    print()
    print("OPTIONS:")
    print("  -d, --directory       Specify the bot's OUTPUT directory.")
    print("  -h, --help            Display this help page and exit.")
    print("  -l, --logging         ENABLE bot operation logging.")
    print("  -s, --stats           DISABLE the nifty stats collection.")
    print("  -v, --verbose         DISABLE exception trace suppression.")
    print()
    print("MODULE: Specify a bot module, located in ./bots")
    print()

def exception_handler(exception_type, exception, traceback):
    print(f"{exception_type.__name__}: {exception}", True)

def parse_args(arg_list):
    arg_directory = "./"      # Specify the output directory
    arg_logging = False        # If true, show bot output
    arg_stats = True           # If true, collect and show the nifty stats
    arg_verbose = False        # If true, show exception trace information
    arg_module = ""

    flags_valid = True
    # module_exists = False

    # Attempt to parse script arguments
    # TODO : Change directory to a bot output directory (if it takes screenshots or the like)
    # TODO : Support multiple modules and allow a flag to specify the game index
    # TODO : Figure out support for multiple bots on multiple clients
    #        (for example, for use with team fights)
    for arg in arg_list[1:]:
        # Last flag was -d or --directory
        if not arg_directory:
            arg_directory = arg
            continue
        # Check flags
        if arg[0] == '-':
            if arg == "-h" or arg == "--help":
                usage()
                exit(0)
            elif arg == "-d" or arg == "--directory": arg_directory = None  # Next arg will be directory
            elif arg == "-l" or arg == "--logging": arg_logging = True
            elif arg == "-s" or arg == "--stats": arg_stats = False
            elif arg == "-v" or arg == "--verbose": arg_verbose = True
            else: flags_valid = False
        # Else implicitly treat arg as module name
        elif arg_module != "": flags_valid = False
        else: arg_module = arg

    # If specified, suppress the stack trace on error
    if not arg_verbose: excepthook = exception_handler

    # Raise error for invalid flags (after verbose so it can show a stacktrace or not)
    if not flags_valid:
        raise Exception("Invalid arguments! Use -h or --help for usage.")

    # Get a module name if not specified
    if arg_module == "":
        # log("Please specify a bot module: ", True)
        arg_module = input("Please specify a bot module: ")

        # For no input on reponse
        if arg_module == "": arg_module = "debug"

    # Checks for '.py' extension of specified module
    if len(arg_module) >= 4 and arg_module[-3:] == ".py":
        arg_module = arg_module[:-3]

    # Checks for invalid characters in name of specified module
    # Possible TODO: allow user to specify location of bot module
    if '\\' in arg_module or '/' in arg_module or ':' in arg_module or '?' in arg_module or '\"' in arg_module or '<' in arg_module or '>' in arg_module or '|' in arg_module or '*' in arg_module or '.' in arg_module or ',' in arg_module or '\#' in arg_module:
        raise Exception("Invalid module! Module name cannot contain the characters: \\ / : * ? \" < > | . , #")

    # Change the working directory to that of the script
    if not arg_directory: arg_directory = "./"

    output = path.abspath(arg_directory)
    if not path.exists(output): raise Exception("Specified directory does not exist!")
    else:
        chdir(output)
        log(f"Output directory is {output}", arg_logging)

    return ArgList(arg_directory, arg_logging, arg_stats, arg_verbose, arg_module)
