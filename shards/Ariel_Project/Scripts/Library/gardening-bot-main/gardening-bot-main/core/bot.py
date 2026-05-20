from core.gardener import Gardener
from core.navigator import Navigator
from utils.actions import Actions
from utils.config_loader import ConfigLoader
import time as t

ConfigLoader.load_config()

class Bot:
    def __init__(self):
        self.actions = Actions()
        self.Images = ConfigLoader.IMAGES
        self.Locations = ConfigLoader.LOCATIONS
        self.Orders = ConfigLoader.ORDERS
        self.Plant_Info = ConfigLoader.PLANT_INFO
        self.Navigator = Navigator(self.actions, self.Images, self.Locations)
        self.Gardener = Gardener(
            self.actions, 
            self.Images, 
            self.Orders, 
            self.Locations, 
            self.Navigator
        )

    def run(self):
        t.sleep(2)
        print("Gardening bot is starting...")
        self.Navigator.go_home()
        self.Gardener.garden(
            self.Orders.COUCH_POTATOES,
            self.Locations.FIRST_GARDEN,
            self.Plant_Info.COUCH_POTATOES
        )
