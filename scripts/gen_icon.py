import sys
import subprocess

try:
    from PIL import Image, ImageDraw
except ImportError:
    subprocess.check_call([sys.executable, "-m", "pip", "install", "pillow", "--quiet"])
    from PIL import Image, ImageDraw
import math

size = 1024
img = Image.new('RGBA', (size, size), (0, 0, 0, 0))
draw = ImageDraw.Draw(img)

def draw_hexagon(draw, center, radius, fill, outline, width):
    points = []
    for i in range(6):
        angle = math.pi / 3 * i - math.pi / 2
        points.append((center[0] + radius * math.cos(angle), center[1] + radius * math.sin(angle)))
    draw.polygon(points, fill=fill, outline=outline, width=width)

# Outer Hexagon (Blackish background, deep blue border)
draw_hexagon(draw, (512, 512), 480, fill="#09090b", outline="#0ea5e9", width=40)
# Inner Hexagon (Slightly lighter panel color, bright blue border)
draw_hexagon(draw, (512, 512), 360, fill="#18181b", outline="#38bdf8", width=20)
# Core dot
draw.ellipse((460, 460, 564, 564), fill="#0ea5e9")

img.save("D:\\Aaroneous\\MaelstromUI\\app-icon.png")
print("Icon generated at D:\\Aaroneous\\MaelstromUI\\app-icon.png")