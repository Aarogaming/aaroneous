###==========================================================================###
### module.py - by Drayux                                                    ###
### MODULE ABSTRACT CLASS                                                    ###
### Defines abstract class for wizard automatus bots                         ###
###==========================================================================###

# Native imports
from abc import ABC, abstractmethod

# Local imports
import func.util

import lib.statistic

# Third-party imports
import asyncio
import wizwalker

class Module(ABC):
    @classmethod
    def initialize(cls, logging, stats, directory):
        # This check might not be necessary
        if not issubclass(cls, Module):   # Could also add (cls == Module) check
            raise Exception("Module is not a proper child class")
        return cls(logging, stats, directory)

    def __init__(self, logging, stats, directory):
        # Program arguments
        self.logging = logging
        self.stats = stats
        self.directory = directory

        # Bot objects
        self.client = None
        self.walker = wizwalker.WizWalker()
        self.hotkeys = []
        self.stats = lib.statistic.Statistic(logging) if stats else None

        # Member vars for program flow
        # IF paused AND should_exit, exit immediately
        self.paused = False            # Checked by bot functions
        self.should_exit = False       # Checked by program loop

    def log(self, message):
        func.util.log(message, self.logging)

    # UTILITY FUNCTIONS
    def register_hotkey(self, keycode, function, modifierkeys=0):
        # hotkey = wizwalker.Hotkey(keycode, function)
        hotkey = wizwalker.Hotkey(keycode, function, modifiers=modifierkeys)
        listener = wizwalker.Listener(hotkey)

        self.log(f"Registering hotkey {keycode}")

        self.hotkeys.append(listener)
        listener.listen_forever()

    def update_stat(self, stat, amount=1):
        if self.stats: self.stats.update(stat, amount)

    # ABSTRACT METHODS (For bot behavior)
    @abstractmethod
    async def before(self):
        pass

    @abstractmethod
    async def loop(self):
        pass

    @abstractmethod
    async def after(self):
        pass

    # PROGRAM FLOW FUNCTIONS
    async def handle_loop(self):
        self.log("Executing primary loop...")
        while not self.should_exit:
            self.update_stat(lib.statistic.StatCategory.LOOP_COUNT)

            await self.handle_pause()
            await self.loop()

            await asyncio.sleep(1)
        await self.after()
        self.paused = True

    async def handle_pause(self):
        was_paused = False
        while self.paused:
            if not was_paused: self.log("Paused")
            was_paused = True

            self.update_stat(lib.statistic.StatCategory.TIME_PAUSED)
            await asyncio.sleep(1)
        if was_paused: self.log("Resuming")

    async def handle_exit(self):
        while True:
            # print(self.stats.data[lib.statistic.StatCategory.LOOP_COUNT])
            if self.paused and self.should_exit:
                return
            await asyncio.sleep(1)

    # HOTKEY CALLBACK FUNCTIONS
    async def callback_pause(self):
        # If should exit is set, pausing should immediately exit, so unpausing is disabled
        if not self.paused: self.paused = True
        elif not self.should_exit: self.paused = False

    async def callback_exit(self):
        self.should_exit = True
