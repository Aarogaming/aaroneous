###==========================================================================###
### stats.py - by Drayux                                                     ###
### MODULE ABSTRACT CLASS                                                    ###
### Defines abstract class for wizard automatus bots                         ###
###==========================================================================###

# Native imports
from enum import Enum

# Local imports
from func.util import log

# Add all new stats here
class StatCategory(Enum):
    DEBUG = 0             # A generic value for development
    LOOP_COUNT = 1        # The total number of times the primary loop has been called
    TIME_PAUSED = 2       # The total amount of time the bot has spent idle
    TELEPORTS = 3         # Total number of times the player has teleported the client

class Statistic():
    def __init__(self, logging):
        self.data = {}
        self.logging = logging

    def update(self, stat, amount):
        # Checks that element is an enum type
        if not isinstance(stat, StatCategory):
            raise TypeError(f"Invalid stat category, {stat} is not an enumerated type")

        # Updates the dictionary value
        if stat in self.data:
            val = self.data[stat]
            val += amount
            self.data.update({stat: val})
        else: self.data.update({stat: amount})

        # Outputs changes if specified
        log(f"Updated {stat.name} to {self.data[stat]}", self.logging)
