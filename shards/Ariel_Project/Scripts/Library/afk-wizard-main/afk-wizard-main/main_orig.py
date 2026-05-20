from random import randint
import pyautogui as auto
from time import sleep

def find_image(image_path):
	print(f'Looking for an {image_path}...')
	img = None
	while(img == None):
		img = auto.locateOnScreen(image_path, confidence=0.7, grayscale=True)
	sleep(0.5)
	return img

def click_image(image_path):
    print("Clicking image ...")
    img = find_image(image_path)
    button = auto.center(img)
    auto.click(button)
    auto.click(button)
    auto.move(0,200)

def in_battle():
	img = auto.locateOnScreen("images/book.jpg", confidence=0.7, grayscale=True)
	if ( img == None):
		return True
	else:
		return False

def wander():
    for i in range(0,4):
        key_choice = randint(0,4)
        step_time = randint(1,2)
        key1 = None
        key2 = None
        if key_choice == 0:
            key1 = "W"
            key2 = "S"
        elif key_choice == 1:
            key1 = "W"
            key2 = "S"
        elif key_choice == 2:
            key1 = "S"
            key2 = "W"
        elif key_choice == 3:
            key1 = "A"
            key2 = "D"
        elif key_choice == 4:
            key1 = "D"
            key2 = "A"
        # walk
        with auto.hold(key1):
            auto.sleep(step_time)
        with auto.hold(key2):
            auto.sleep(step_time)
    print("Wander complete")

# program begins
print("Switch window focus to Wizard101. Farming will begin in 5 seconds")
sleep(5)
while(True):
    if(in_battle()):
        click_image(image_path="images/pass.jpg")
        click_image(image_path="images/spell.jpg")
        # wait for book to be visible
        while(in_battle()):
	        sleep(0.1)
        wander()
