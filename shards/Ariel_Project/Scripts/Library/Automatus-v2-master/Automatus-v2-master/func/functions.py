###==========================================================================###
### functions.py - by Drayux                                                 ###
### BOT OPERATION FUNCTIONS                                                  ###
### A collection of useful functions for use with bots, excluding battle     ###
###==========================================================================###

# Native imports
import math
import os
import random

# Local imports
from func.util import log
from lib.statistic import StatCategory

# Third-party imports
import asyncio
from wizwalker import Keycode, WIZARD_SPEED, XYZ

# Prints debug information about the player/game
async def debug_info(bot):
    p_zone = await bot.client.zone_name()
    p_pos = await bot.client.body.position()
    p_yaw = await bot.client.body.yaw()

    print("======================== DEBUG INFO ========================")
    log(f"Zone : {p_zone}", True)
    log(f"Position : {round(p_pos.x)}, {round(p_pos.y)}, {round(p_pos.z)}", True)
    log(f"Rotation : {round(p_yaw, 2)} rad  ({round(p_yaw * 180 / 3.14159)} deg)", True)
    print("============================================================")


# Experimental way to noclip walls
async def noclip_forward(bot, speed=0.1):
    p_pos = await bot.client.body.position()
    p_yaw = await bot.client.body.yaw()

    cur_x = p_pos.x
    cur_y = p_pos.y

    delta_x = -1 * WIZARD_SPEED * speed * math.sin(p_yaw)
    delta_y = -1 * WIZARD_SPEED * speed * math.cos(p_yaw)

    fin_x = cur_x + delta_x
    fin_y = cur_y + delta_y

    log(f"Teleporting client to {fin_x}, {fin_y}", bot.logging)
    bot.update_stat(StatCategory.TELEPORTS)

    await bot.client.teleport(XYZ(fin_x, fin_y, p_pos.z))
    await asyncio.sleep(speed)

async def noclip_up(bot, speed=0.1):
    p_pos = await bot.client.body.position()

    cur_z = p_pos.z
    delta_z = WIZARD_SPEED * speed
    fin_z = cur_z + delta_z

    log(f"Teleporting client to {fin_z}", bot.logging)
    bot.update_stat(StatCategory.TELEPORTS)

    await bot.client.teleport(XYZ(p_pos.x, p_pos.y, fin_z))
    await asyncio.sleep(speed)

# Creates a task that prevents the player from being AFK kicked
#   (so long as they are in a position where movement inputs are registered)
# THIS FUNCTION DOES NOT PAUSE
async def anti_afk(bot):
    async def get_position():
        p_pos = await bot.client.body.position()
        return (round(p_pos.x), round(p_pos.y))

    async def afk_loop(p_bot):
        range = 24        # (120 / 5 = ) 2 minutes
        offset = 36       # (180 / 5 = ) 3 minutes

        time_waited = 0
        time_to_wait = 60
        last_position = (0, 0)

        while not bot.should_exit:
            # Check player position
            cur_position = await get_position()
            if last_position != cur_position: time_waited = 0
            elif time_waited >= time_to_wait:
                # Player has gone AFK
                log("Resetting AFK timer...", bot.logging)
                await p_bot.client.send_key(Keycode.D, 1.0)
                await p_bot.client.send_key(Keycode.A, 0.9)
                # Send move right
                time_waited = 0
                time_to_wait = (random.randint(0, range) + offset) * 5
                log(f"New wait time: {time_to_wait} seconds", bot.logging)
                last_position = cur_position

            last_position = cur_position
            time_waited += 5
            await asyncio.sleep(5)

    asyncio.create_task(afk_loop(bot))
