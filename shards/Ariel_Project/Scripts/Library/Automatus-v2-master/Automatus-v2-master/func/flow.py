###==========================================================================###
### flow.py - by Drayux                                                      ###
### SCRIPT FLOW CONTROL                                                      ###
### Manages the asynchronous script flow beyond that specified in main       ###
###==========================================================================###

### TODO ###
# IF client not detected:
# Automatically log user in and launch game IF credentials are provided

# PAUSE / STOP FUNCTIONALITY

# Local imports
from func.util import log

# Third-party imports
import asyncio

async def bot_main(player_bot):
    try:
        # ---SETUP PHASE (before)---
        # Attach a game executable to the bot object
        client = None
        clients = player_bot.walker.get_new_clients()
        if len(clients) > 0:
            # TODO try to hook to second client if first already hooked
            client = clients[0]
            await client.activate_hooks()
        else:
            # TODO automatic launch functionality
            pass # raise NotImplementedError("No wizard101 clients detected!")

        if client: player_bot.client = client
        else: pass # raise Exception("Failed to hook game client!")

        log("Successfully hooked module to client!", player_bot.logging)
        log("Executing bot setup...", player_bot.logging)

        await player_bot.before()
        if not player_bot.logging: log("Welcome to Wizard Automatus!", True)

        # ---LOOP PHASE (loop)---
        # Execute bot on secondary task that can be closed here
        asyncio.create_task(player_bot.handle_loop())
        await asyncio.create_task(player_bot.handle_exit())

    finally:
        # ---FINISH PHASE (after)---
        print("TODO complete stats system")

        log("Unhooking Wizard101 client...", player_bot.logging)
        await player_bot.walker.close()

        # This order is important
        log("Removing hotkeys...", player_bot.logging)
        for hotkey in player_bot.hotkeys:
            await hotkey.close()
            # log(f"Closed hotkey: {hotkey}", player_bot.logging)
