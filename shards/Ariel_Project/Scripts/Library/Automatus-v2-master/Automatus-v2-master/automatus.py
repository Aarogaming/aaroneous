###==========================================================================###
### automatus.py - by Drayux                                                 ###
### MAIN FILE FOR BOT (run this file with the python command)                ###
### Handles bot setup and top-level bot loop                                 ###
###==========================================================================###

# Native imports
import os
import sys

# Local imports
from func.args import parse_args
from func.flow import bot_main
from func.util import *

from lib.module import Module

# Third-party imports
import asyncio
import wizwalker

if __name__ == "__main__":
    # Parse script arguments
    args = parse_args(sys.argv)
    log(args, args.verbose)
    log("Logging enabled", args.logging)

    # Attempt to load specified module
    module_valid = True
    exec(f"import bots.{args.module} as module")
    exec(f"from bots.{args.module} import {module.CLASS_NAME} as bot")
    # try:
    #     exec(f"import bots.{args.module} as module")
    #     exec(f"from bots.{args.module} import {module.CLASS_NAME} as bot")
    # except Exception:
    #     module_valid = False
    # # TODO Display more error information - this is called whenever there is an error in the bot definition
    # if not module_valid: raise Exception(f"Failed to load specified module: {args.module}")

    log(f"Successfully loaded {args.module}!", args.logging)

    # Create bot (unlinked to a game client)
    p_bot = bot.initialize(args.logging, args.stats, args.directory)

    # TODO - WAITING ON THIS TO BE FIXED IN THE LIBRARY
    wizwalker.utils.override_wiz_install_location("C:\\Games\\Wizard101")

    # Begin asyncio loop
    asyncio.run(bot_main(p_bot))
