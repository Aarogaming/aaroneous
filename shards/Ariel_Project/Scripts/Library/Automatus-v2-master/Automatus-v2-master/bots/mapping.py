###==========================================================================###
### mapping.py - by Drayux                                                   ###
### BOT MODULE --> MAPPING                                                   ###
### Bot for direct use with locomotion functionalty, and constructing data   ###
###==========================================================================###

# Local imports
from func.functions import *
from func.locomotion import *
from lib.module import Module
from lib.map import *

# Third-party imports
import asyncio
import wizwalker

from wizwalker import Hotkey, Keycode, Listener, ModifierKeys

# DEFINE BOT CONFIGURATION
# I wish there was a better way to do this, but I'm not certain that there is
CLASS_NAME = "Mapping"
WIZARD_NAME = ""

class Mapping(Module):
    # Executed once before the bot loops
    async def before(self):
        # REGISTER HOTKEYS
        self.register_hotkey(Keycode.ONE, self.callback_debug, ModifierKeys.ALT)
        self.register_hotkey(Keycode.TWO, self.callback_set_point, ModifierKeys.ALT)
        self.register_hotkey(Keycode.THREE, self.callback_undo_point, ModifierKeys.ALT)
        self.register_hotkey(Keycode.FOUR, self.callback_set_W, ModifierKeys.ALT)
        self.register_hotkey(Keycode.FIVE, self.callback_set_S, ModifierKeys.ALT)
        self.register_hotkey(Keycode.SIX, self.callback_set_X, ModifierKeys.ALT)
        self.register_hotkey(Keycode.SEVEN, super().callback_exit, ModifierKeys.ALT)

        await anti_afk(self)

        # testmap = Map("WizardCity", "UNICORN_WAY")
        # await follow_map(self, testmap, forward=True)
        # await append_point(self, XYZ(0,0,1))
        # await remove_point(self)

    # Executed indefinitely (until stopped - 1 second sleep between iterations)
    async def loop(self):
        pass # Specify self.should_exit = True if you don't want the bot to loop

    # Executed once after bot is requested to exit
    async def after(self):
        testmap = Map("Custom", "USER_REC")
        await follow_map(self, testmap, forward=False)

    # HOTKEY FUNCTIONS
    async def callback_debug(self):
        await debug_info(self)

    async def callback_set_point(self):
        p_pos = await self.client.body.position()
        await append_point(self, p_pos)

    async def callback_undo_point(self):
        await remove_point(self)

    async def callback_set_W(self):
        await append_point(self, XYZ(0,0,1))

    async def callback_set_S(self):
        await append_point(self,  XYZ(0,0,0))

    async def callback_set_X(self):
        await append_point(self,  XYZ(0,0,-1))
