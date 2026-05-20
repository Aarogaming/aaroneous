###==========================================================================###
### util.py - by Drayux                                                      ###
### MISCELLANEOUS SCRIPT FUNCTIONS                                           ###
### Contains functions that will be used globally throughout the script      ###
###==========================================================================###

import time

def timestamp():
    cur_time = time.localtime(time.time())
    return f"{cur_time.tm_hour}:{cur_time.tm_min}"

def log(message, should_output):
    if should_output: print(f"[Automatus][{timestamp()}] {message}")
