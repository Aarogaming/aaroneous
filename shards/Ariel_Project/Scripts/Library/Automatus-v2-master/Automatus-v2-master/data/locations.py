###==========================================================================###
### locations.py - by Drayux                                                 ###
### GAME LOCATION TREE LAYOUT                                                ###
### Defines the structure of a tree that models map locations in the game    ###
###==========================================================================###

# Native imports
from enum import Enum

# class World(Enum):
#     WIZARD_CITY = 1
#     KROKOTOPIA = 2
#     MARLEYBONE = 3
#     MOOSHU = 4
#     DRAGONSPYRE = 5
#     GRIZZLEHEIM = 6
#     CELESTIA = 7
#     WYSTERIA = 8
#     ZAFARIA = 9
#     AVALON = 10
#     AZTECA = 11
#     KHRYSALIS = 12
#     POLARIS = 13
#     ARCANUM = 14
#     MIRAGE = 15
#     EMPYREA = 16
#     KARAMELLE = 17

FILE_NAMES = {
    # Wizard City
    "WizardCity/WC_Hub" : "COMMONS",
    "WizardCity/WC_Ravenwood" : "RAVENWOOD",
    "WizardCity/Interiors/WC_SchoolFire" : "SCHOOL_OF_FIRE",
    "WizardCity/Interiors/WC_TowerFire" : "FIRE_TOWER",
    "WizardCity/Interiors/WC_SchoolFrost" : "SCHOOL_OF_ICE",
    "WizardCity/Interiors/WC_TowerIce" : "ICE_TOWER",
    "WizardCity/Interiors/WC_SchoolStorm" : "SCHOOL_OF_STORM",
    "WizardCity/Interiors/WC_TowerStorm" : "STORM_TOWER",
    "WizardCity/Interiors/WC_SchoolLife" : "SCHOOL_OF_LIFE",
    "WizardCity/Interiors/WC_TowerLife" : "LIFE_TOWER",
    "WizardCity/Interiors/WC_SchoolMyth" : "SCHOOL_OF_MYTH",
    "WizardCity/Interiors/WC_TowerMyth" : "MYTH_TOWER",
    "WizardCity/WC_Ravenwood_Teleporter" : "WORLD_TREE",
    "WizardCity/WC_NightSide" : "NIGHTSIDE",
    "WizardCity/Interiors/WC_SchoolDeath" : "SCHOOL_OF_DEATH",
    "WizardCity/Interiors/WC_TowerDeath" : "DEATH_TOWER",
    "G14_DM/DM_Z01_CastleDarkmoor" : "DARKMOOR",
    "WizardCity/WC_Streets/WC_Unicorn" : "UNICORN_WAY",
    "WizardCity/WC_Duel_Arena" : "ARENA",
    "WizardCity/WC_Streets/Interiors/WC_Unicorn_HedgeMaze" : "HEDGE_MAZE",
    "WizardCity/Interiors/WC_Library" : "LIBRARY",
    "WizardCity/Interiors/WC_Castle_Tours" : "CASTLE_TOURS",
    "WizardCity/WC_Streets/Interiors/WC_PET_Park" : "PET_PAVILION",
    "WizardCity/Interiors/WC_Hatchery" : "HATCHERY",
    "WizardCity/Interiors/WC_Headmistress_House" : "HEADMASTERS_HOUSE",
    "WizardCity/Interiors/WC_Headmaster_Tower" : "HEADMASTERS_TOWER",
    "WizardCity/WC_Golem_Tower" : "GOLEM_COURT",
    "WizardCity/WC_Streets/WC_Drains/Z00_Drains_HUB" : "DRAINS",
    "WizardCity/WC_Streets/WC_Catacombs" : "CATACOMBS",
    "WizardCity/WC_Streets/WC_Catacombs_B" : "LOWER_CATACOMBS",
    "WizardCity/WC_Shop_Area" : "SHOPPING_DISTRICT",
    "WizardCity/Interiors/WC_Shop_Makeup" : "MIRROR_SHOP",
    "WizardCity/Interiors/WC_ShopDye" : "DYE_SHOP",
    "WizardCity/Interiors/WC_Shop_Hat" : "HAT_SHOP",
    "WizardCity/Interiors/WC_Shop_Pets" : "PET_SHOP",
    "WizardCity/Interiors/WC_Shop_Boots" : "BOOT_SHOP",
    "WizardCity/Interiors/WC_Shop_Decks" : "DECK_SHOP",
    "WizardCity/Interiors/WC_Shop_House" : "HOUSING_SHOP",
    "WizardCity/Interiors/WC_Shop_Rings" : "RING_SHOP",
    "WizardCity/Interiors/WC_Shop_Athames" : "ATHAME_SHOP",
    "WizardCity/Interiors/WC_Shop_Jeweler" : "JEWEL_SHOP",
    "WizardCity/Interiors/WC_Shop_Robes" : "ROBE_SHOP",
    "WizardCity/Interiors/WC_Shop_Wand" : "WAND_SHOP",
    "WizardCity/Interiors/WC_Shop_Amulets" : "AMULET_SHOP",
    "WizardCity/WC_Streets/WC_Colossus" : "COLOSSUS_BOULEVARD",
    "WizardCity/WC_Streets/Interiors/WC_Colossus_H1" : "MILDREDS_HOUSE",
    "WizardCity/WC_Streets/WC_OldeTown" : "OLDE_TOWN",
    "WizardCity/WC_Streets/Interiors/WC_OldeTown_AuctionHouse" : "BAZAAR",
    "WizardCity/WC_Streets/WC_Cyclops" : "CYCLOPS_LANE",
    "Aquila/AQ_Z00_Hub" : "AQUILA",
    "WizardCity/WC_Streets/WC_Triton" : "TRITON_AVENUE",
    "WizardCity/WC_Streets/WC_HauntedCave" : "HAUNTED_CAVE",
    "WizardCity/WC_Streets/WC_Firecat" : "FIRECAT_ALLEY",
    "WizardCity/WC_Streets/Interiors/WC_Firecat_H2" : "GRETTAS_HOUSE",
    "WizardCity/WC_Streets/WC_DarkCave" : "DARK_CAVE"
}

LOCATIONS = {
    "WizardCity" : {
        "name" : "WizardCity/WC_Hub",
        "children" : [{
            "name" : "WizardCity/WC_Ravenwood",
            "children" : [{
                "name" : "WizardCity/Interiors/WC_SchoolFire",
                "children" : []
            }, {
                "name" : "WizardCity/Interiors/WC_TowerFire",
                "children" : []
            }, {
                "name" : "WizardCity/Interiors/WC_SchoolFrost",
                "children" : []
            }, {
                "name" : "WizardCity/Interiors/WC_TowerIce",
                "children" : []
            }, {
                "name" : "WizardCity/Interiors/WC_SchoolStorm",
                "children" : []
            }, {
                "name" : "WizardCity/Interiors/WC_TowerStorm",
                "children" : []
            }, {
                "name" : "WizardCity/Interiors/WC_SchoolLife",
                "children" : []
            }, {
                "name" : "WizardCity/Interiors/WC_TowerLife",
                "children" : []
            }, {
                "name" : "WizardCity/Interiors/WC_SchoolMyth",
                "children" : []
            }, {
                "name" : "WizardCity/Interiors/WC_TowerMyth",
                "children" : []
            }, {
                "name" : "WizardCity/WC_Ravenwood_Teleporter",
                "children" : []
            }]
        }, {
            "name" : "WizardCity/WC_NightSide",
            "children" : [{
                "name" : "WizardCity/Interiors/WC_SchoolDeath",
                "children" : []
            }, {
                "name" : "WizardCity/Interiors/WC_TowerDeath",
                "children" : []
            }, {
                "name" : "G14_DM/DM_Z01_CastleDarkmoor",
                "children" : []
            }]
        }, {
            "name" : "WizardCity/WC_Streets/WC_Unicorn",
            "children" : [{
                "name" : "WizardCity/WC_Duel_Arena",
                "children" : []
            }, {
                "name" : "WizardCity/WC_Streets/Interiors/WC_Unicorn_HedgeMaze",
                "children" : []
            }]
        }, {
            "name" : "WizardCity/Interiors/WC_Library",
            "children" : []
        }, {
            "name" : "WizardCity/Interiors/WC_Castle_Tours",
            "children" : []
        }, {
            "name" : "WizardCity/WC_Streets/Interiors/WC_PET_Park",
            "children" : [{
                "name" : "WizardCity/Interiors/WC_Hatchery",
                "children" : []
            }]
        }, {
            "name" : "WizardCity/Interiors/WC_Headmistress_House",
            "children" : []
        }, {
            "name" : "WizardCity/Interiors/WC_Headmaster_Tower",
            "children" : []
        }, {
            "name" : "WizardCity/WC_Golem_Tower",
            "children" : [{
                "name" : "WizardCity/WC_Streets/WC_Drains/Z00_Drains_HUB",
                "children" : [{
                    "name" : "WizardCity/WC_Streets/WC_Catacombs",
                    "children" : [{
                        "name" : "WizardCity/WC_Streets/WC_Catacombs_B",
                        "children" : []
                    }]
                }]
            }]
        }, {
            "name" : "WizardCity/WC_Shop_Area",
            "children" : [{
                "name" : "WizardCity/WC_Streets/WC_Colossus",
                "children" : [{
                    "name" :  "WizardCity/WC_Streets/Interiors/WC_Colossus_H1",
                    "children" : []
                }]
            }, {
                "name" : "WizardCity/WC_Streets/WC_OldeTown",
                "children" : [{
                    "name" : "WizardCity/WC_Streets/Interiors/WC_OldeTown_AuctionHouse",
                    "children" : []
                }, {
                    "name" : "WizardCity/WC_Streets/WC_Cyclops",
                    "children" : [{
                        "name" : "WizardCity/WC_Streets/WC_DarkCave",
                        "children" : []
                    }, {
                        "name" : "Aquila/AQ_Z00_Hub",
                        "children" : []
                    }]
                }, {
                    "name" : "WizardCity/WC_Streets/WC_Triton",
                    "children" : [{
                        "name" : "WizardCity/WC_Streets/WC_HauntedCave",
                        "children" : []
                    }]
                }, {
                    "name" : "WizardCity/WC_Streets/WC_Firecat",
                    "children" : [{
                        "name" :  "WizardCity/WC_Streets/Interiors/WC_Firecat_H2",
                        "children" : []
                    }]
                }]
            }, {
                "name" : "WizardCity/Interiors/WC_Shop_Makeup",
                "children" : []
            }, {
                "name" : "WizardCity/Interiors/WC_ShopDye",
                "children" : []
            }, {
                "name" : "WizardCity/Interiors/WC_Shop_Hat",
                "children" : []
            }, {
                "name" : "WizardCity/Interiors/WC_Shop_Pets",
                "children" : []
            }, {
                "name" : "WizardCity/Interiors/WC_Shop_Boots",
                "children" : []
            }, {
                "name" : "WizardCity/Interiors/WC_Shop_Decks",
                "children" : []
            }, {
                "name" : "WizardCity/Interiors/WC_Shop_House",
                "children" : []
            }, {
                "name" : "WizardCity/Interiors/WC_Shop_Rings",
                "children" : []
            }, {
                "name" : "WizardCity/Interiors/WC_Shop_Athames",
                "children" : []
            }, {
                "name" : "WizardCity/Interiors/WC_Shop_Jeweler",
                "children" : []
            }, {
                "name" : "WizardCity/Interiors/WC_Shop_Robes",
                "children" : []
            }, {
                "name" : "WizardCity/Interiors/WC_Shop_Wand",
                "children" : []
            }, {
                "name" : "WizardCity/Interiors/WC_Shop_Amulets",
                "children" : []
            }]
        }]
    },
    "Krokotopia" : {}
}
