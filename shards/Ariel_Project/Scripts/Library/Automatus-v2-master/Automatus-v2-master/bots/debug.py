###==========================================================================###
### debug.py - by Drayux                                                     ###
### BOT MODULE --> DEBUG                                                     ###
### Development bot with functions for information and OOB movement          ###
###==========================================================================###

# Local imports
from func.functions import *
from lib.module import Module
from lib.map import *

# Third-party imports
import asyncio
import wizwalker

from wizwalker import Hotkey, Keycode, Listener, ModifierKeys

# DEFINE BOT CONFIGURATION
# I wish there was a better way to do this, but I'm not certain that there is
CLASS_NAME = "Debug"
WIZARD_NAME = ""

class Debug(Module):
    # Executed once before the bot loops
    async def before(self):
        # REGISTER DEBUG HOTKEYS - OPTIONAL
        self.register_hotkey(Keycode.ONE, self.callback_debug, ModifierKeys.ALT)
        self.register_hotkey(Keycode.TWO, super().callback_pause, ModifierKeys.ALT)
        self.register_hotkey(Keycode.THREE, super().callback_exit, ModifierKeys.ALT)

        # REGISTER NOCLIP HOTKEYS - OPTIONAL
        self.register_hotkey(Keycode.FOUR, self.callback_noclip_forward, ModifierKeys.ALT)
        self.register_hotkey(Keycode.FIVE, self.callback_noclip_up, ModifierKeys.ALT)
        self.register_hotkey(Keycode.SIX, self.callback_noclip_down, ModifierKeys.ALT)

        # TURN ON ANTI-AFK
        await anti_afk(self)

    # Executed indefinitely (until stopped - 1 second sleep between iterations)
    async def loop(self):
        pass # Specify 'self.should_exit = True' if you don't want the bot to loop

    # Executed once after bot is requested to exit
    async def after(self):
        pass

    # HOTKEY FUNCTIONS - OPTIONAL
    async def callback_debug(self):
        await debug_info(self)

    async def callback_noclip_forward(self):
        await noclip_forward(self)

    async def callback_noclip_up(self):
        await noclip_up(self)

    async def callback_noclip_down(self):
        await noclip_up(self, -0.1)
