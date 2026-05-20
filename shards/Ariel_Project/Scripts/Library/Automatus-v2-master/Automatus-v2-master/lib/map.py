###==========================================================================###
### map.py - by Drayux                                                       ###
### POINT MAP CLASS                                                          ###
### Defines the point map data structure                                     ###
###==========================================================================###

from data.locations import *
from func.util import log
from wizwalker import XYZ

root_dir = "./data/maps"

def read_points(file_location, locname):
    points = []

    try:
        file = open(file_location, "r")
        point_list = file.readlines()

        # Filename should be specified in line one
        if locname != point_list[0].strip():
            log("Invalid map file format: Names do not match!", True)
            raise Exception("Invalid map file format")

        # Parse point map into set of tuples
        for pointstr in point_list[1:]:
            point = pointstr.split(',')
            points.append(XYZ(int(point[0]), int(point[1]), int(point[2])))

        file.close()

    except Exception:
        log(f"Failed to open map file: {locname}!", True)

    finally:
        return points

class Map():
    # TODO - USE WORLDNAME TO CHECK IF WORLD MATCHES AND GOTO DIFFERENT WORLD IF NOT
    # Worldname --> Key at first level of locations.LOCATIONS
    # Locname --> Value in locations.LOCATION_NAMES
    def __init__(self, worldname, locname):
        self.locname = locname
        self.worldname = worldname
        self.file_location = f"{root_dir}/{worldname}/{locname}"
        self.data = read_points(self.file_location, locname)
        self.index = 0                                # Current position of the iterator

        self.first = 0                                # Index of the first point (always 0)
        self.last = len(self.data) - 1                # Index of the last point

        self.start = True                             # Iterator is at beginning of array
        self.end = False if self.last > 1 else True   # Iterator is at end of array

    # Return current point and iterate to next point
    def next(self):
        value = self.data[self.index]

        if self.index == self.last: self.end = True
        else:
            self.start = False
            self.index += 1

        return value

    # Return current point and iterate to previous point
    def prev(self):
        value = self.data[self.index]

        if self.index == self.first: self.start = True
        else:
            self.end = False
            self.index -= 1

        return value

    def goto_start(self):
        self.index = self.first
        self.start = True
        self.end = False

    def goto_end(self):
        self.index = self.last
        self.start = False
        self.end = True
