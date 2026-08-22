import math
import numpy as np
from PIL import Image, ImageDraw, ImageFilter

SIZE = 1024
img = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))

# ── 1. Create Apple macOS Squircle Mask ─────────────────────────
# macOS icon squircle formula: |x/a|^n + |y/b|^n <= 1 with n ~ 4.5 or 5
# Icon size in standard 1024 canvas is approx 824x824 centered (padding 100px)
CENTER = SIZE / 2
RADIUS = 410.0
EXPONENT = 4.8  # macOS continuous squircle curvature

y, x = np.ogrid[:SIZE, :SIZE]
dx = np.abs(x - CENTER) / RADIUS
dy = np.abs(y - CENTER) / RADIUS
squircle_dist = (dx ** EXPONENT + dy ** EXPONENT) ** (1.0 / EXPONENT)

# Anti-aliased mask
squircle_mask = np.clip((1.0 - squircle_dist) * 80.0, 0.0, 1.0)

# ── 2. Create Background Gradient & Inner Lighting ─────────────
# Deep midnight navy to electric indigo gradient
bg_arr = np.zeros((SIZE, SIZE, 4), dtype=np.float32)

for row in range(SIZE):
    t = row / float(SIZE)
    # Gradient from #0d111a to #1b1633 to #090c15
    r = (13 * (1 - t) + 27 * t * (1 - t) * 4 + 9 * t)
    g = (17 * (1 - t) + 22 * t * (1 - t) * 4 + 12 * t)
    b = (26 * (1 - t) + 55 * t * (1 - t) * 4 + 25 * t)
    bg_arr[row, :, 0] = r
    bg_arr[row, :, 1] = g
    bg_arr[row, :, 2] = b
    bg_arr[row, :, 3] = 255.0

# Radial ambient cyan/purple glow in the center
dist_from_center = np.sqrt((x - CENTER)**2 + (y - CENTER + 40)**2) / RADIUS
glow_intensity = np.clip(1.0 - dist_from_center, 0.0, 1.0) ** 1.8
bg_arr[:, :, 0] += glow_intensity * 30  # subtle violet
bg_arr[:, :, 1] += glow_intensity * 70  # cyan
bg_arr[:, :, 2] += glow_intensity * 130 # blue

# Apply squircle mask to background
bg_arr[:, :, 3] *= squircle_mask
bg_img = Image.fromarray(np.uint8(np.clip(bg_arr, 0, 255)), mode="RGBA")

# ── 3. Render Inner Graphics on Separate Layers ─────────────────
fg_img = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
draw = ImageDraw.Draw(fg_img)

# A. Glowing Disk / Storage Ring
ring_center = (512, 512)
ring_radius = 260
ring_width = 36

# Ambient glow under ring
glow_layer = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
glow_draw = ImageDraw.Draw(glow_layer)

glow_draw.ellipse(
    [ring_center[0] - ring_radius - 20, ring_center[1] - ring_radius - 20,
     ring_center[0] + ring_radius + 20, ring_center[1] + ring_radius + 20],
    outline=(56, 189, 248, 120), width=60
)
glow_layer = glow_layer.filter(ImageFilter.GaussianBlur(25))
fg_img = Image.alpha_composite(fg_img, glow_layer)
draw = ImageDraw.Draw(fg_img)

# Ring base (dark metallic groove)
draw.ellipse(
    [ring_center[0] - ring_radius, ring_center[1] - ring_radius,
     ring_center[0] + ring_radius, ring_center[1] + ring_radius],
    outline=(30, 41, 59, 255), width=ring_width
)

# Active storage progress arc (glowing neon cyan to violet, from -140 deg to 80 deg)
for angle in range(-140, 90, 2):
    t = (angle + 140) / 230.0
    r = int(56 * (1 - t) + 168 * t)
    g = int(189 * (1 - t) + 85 * t)
    b = int(248 * (1 - t) + 247 * t)
    draw.arc(
        [ring_center[0] - ring_radius, ring_center[1] - ring_radius,
         ring_center[0] + ring_radius, ring_center[1] + ring_radius],
        start=angle, end=angle + 3,
        fill=(r, g, b, 255), width=ring_width
    )

# B. Central Drive Body / Floating Plate (Sleek Rounded Square)
plate_bbox = [340, 340, 684, 684]
plate_layer = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
plate_draw = ImageDraw.Draw(plate_layer)

# Plate Drop Shadow
plate_draw.rounded_rectangle(
    [plate_bbox[0], plate_bbox[1] + 15, plate_bbox[2], plate_bbox[3] + 15],
    radius=48, fill=(0, 0, 0, 160)
)
plate_layer = plate_layer.filter(ImageFilter.GaussianBlur(16))

# Plate body (metallic gradient)
plate_inner = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
plate_inner_draw = ImageDraw.Draw(plate_inner)
plate_inner_draw.rounded_rectangle(plate_bbox, radius=48, fill=(20, 26, 42, 245))

# Top-left bevel highlight
plate_inner_draw.rounded_rectangle(
    [plate_bbox[0] + 2, plate_bbox[1] + 2, plate_bbox[2] - 2, plate_bbox[3] - 2],
    radius=46, outline=(255, 255, 255, 45), width=2
)

# Center Storage Chip with glowing cyan accent
chip_bbox = [430, 430, 594, 594]
plate_inner_draw.rounded_rectangle(chip_bbox, radius=24, fill=(15, 23, 42, 255), outline=(56, 189, 248, 180), width=3)

# Storage grid details
for i in range(460, 570, 26):
    plate_inner_draw.line([(i, 450), (i, 574)], fill=(30, 58, 95, 200), width=3)
    plate_inner_draw.line([(450, i), (574, i)], fill=(30, 58, 95, 200), width=3)

# Center glowing cyan core
plate_inner_draw.rounded_rectangle([488, 488, 536, 536], radius=10, fill=(56, 189, 248, 255))
plate_inner_draw.rounded_rectangle([498, 498, 526, 526], radius=6, fill=(255, 255, 255, 255))

# Combine plate layers
fg_img = Image.alpha_composite(fg_img, plate_layer)
fg_img = Image.alpha_composite(fg_img, plate_inner)

# C. Dynamic Neon Magic Broom / Sweeper (Diagonal overlay from top-right to bottom-left)
broom_layer = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
broom_draw = ImageDraw.Draw(broom_layer)

# Handle (Sleek brushed silver / electric violet stick)
# Points: from (720, 260) to (530, 530)
handle_angle = math.atan2(530 - 260, 530 - 720)

# Glow around handle
broom_draw.line([(730, 250), (525, 545)], fill=(168, 85, 247, 100), width=28)
broom_draw.line([(730, 250), (525, 545)], fill=(224, 231, 255, 255), width=12)
broom_draw.line([(730, 250), (525, 545)], fill=(255, 255, 255, 255), width=6)

# Handle top cap jewel
broom_draw.ellipse([720, 240, 750, 270], fill=(56, 189, 248, 255), outline=(255, 255, 255, 255), width=3)

# Broom head / bristles base
# Bristles spreading across (440, 580) to (340, 680)
bristle_glow = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
bristle_glow_draw = ImageDraw.Draw(bristle_glow)

# Bristles fan lines
for offset in range(-45, 50, 8):
    rad = math.radians(offset)
    end_x = 420 + int(150 * math.cos(math.radians(225) + rad))
    end_y = 620 + int(150 * math.sin(math.radians(225) + rad))
    # Outer glow
    bristle_glow_draw.line([(525, 545), (end_x, end_y)], fill=(56, 189, 248, 140), width=18)
    # Inner bright bristle
    broom_draw.line([(525, 545), (end_x, end_y)], fill=(125, 211, 252, 230), width=6)
    broom_draw.line([(525, 545), (end_x, end_y)], fill=(255, 255, 255, 255), width=2)

bristle_glow = bristle_glow.filter(ImageFilter.GaussianBlur(12))
fg_img = Image.alpha_composite(fg_img, bristle_glow)
fg_img = Image.alpha_composite(fg_img, broom_layer)

# D. Sparkles & Stardust Particles (Magic cleaning effect)
sparkle_layer = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
sparkle_draw = ImageDraw.Draw(sparkle_layer)

def draw_star_sparkle(d, cx, cy, size, color):
    # 4-point star sparkle
    points = [
        (cx, cy - size),
        (cx + size * 0.22, cy - size * 0.22),
        (cx + size, cy),
        (cx + size * 0.22, cy + size * 0.22),
        (cx, cy + size),
        (cx - size * 0.22, cy + size * 0.22),
        (cx - size, cy),
        (cx - size * 0.22, cy - size * 0.22),
    ]
    d.polygon(points, fill=color)
    d.ellipse([cx - size * 0.25, cy - size * 0.25, cx + size * 0.25, cy + size * 0.25], fill=(255, 255, 255, 255))

# Draw major sparkles
draw_star_sparkle(sparkle_draw, 340, 480, 36, (255, 255, 255, 255))
draw_star_sparkle(sparkle_draw, 300, 620, 48, (56, 189, 248, 255))
draw_star_sparkle(sparkle_draw, 420, 710, 32, (168, 85, 247, 255))
draw_star_sparkle(sparkle_draw, 640, 340, 30, (255, 255, 255, 255))
draw_star_sparkle(sparkle_draw, 720, 460, 24, (56, 189, 248, 255))

# Floating stardust dots
dots = [(370, 440, 6), (290, 540, 8), (350, 650, 9), (460, 690, 7), (490, 740, 5), (680, 390, 6), (750, 520, 5)]
for dx, dy, dr in dots:
    sparkle_draw.ellipse([dx - dr, dy - dr, dx + dr, dy + dr], fill=(224, 242, 254, 230))

# Glow under sparkles
sparkle_glow = sparkle_layer.filter(ImageFilter.GaussianBlur(8))
fg_img = Image.alpha_composite(fg_img, sparkle_glow)
fg_img = Image.alpha_composite(fg_img, sparkle_layer)

# ── 4. Apply Squircle Mask to Combined Content & Add Glass Highlight ───
final_content = Image.alpha_composite(bg_img, fg_img)

# Mask everything strictly inside squircle
fc_arr = np.array(final_content, dtype=np.float32)
fc_arr[:, :, 3] *= squircle_mask

# Glass Specular Arc at top
glass_y = np.clip(1.0 - (y - (CENTER - 200)) / 280.0, 0.0, 1.0)
glass_spec = (glass_y ** 2.2) * (squircle_mask ** 2.0) * 45.0
fc_arr[:, :, 0] = np.clip(fc_arr[:, :, 0] + glass_spec, 0, 255)
fc_arr[:, :, 1] = np.clip(fc_arr[:, :, 1] + glass_spec, 0, 255)
fc_arr[:, :, 2] = np.clip(fc_arr[:, :, 2] + glass_spec * 1.2, 0, 255)

# Outer Border Stroke (1.5px subtle metallic rim)
rim_dist = np.abs(squircle_dist - 0.99)
rim_mask = np.clip((1.0 - rim_dist * 40.0), 0.0, 1.0)
rim_grad = np.clip((SIZE - y) / float(SIZE), 0.2, 1.0)
fc_arr[:, :, 0] += rim_mask * 255 * rim_grad * 0.3
fc_arr[:, :, 1] += rim_mask * 255 * rim_grad * 0.35
fc_arr[:, :, 2] += rim_mask * 255 * rim_grad * 0.45

# ── 5. Add Native Drop Shadow Under Squircle ───────────────────
shadow_img = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
shadow_arr = np.zeros((SIZE, SIZE, 4), dtype=np.float32)
# Shadow positioned 20px lower
shadow_dy = np.abs(y - (CENTER + 24)) / RADIUS
shadow_dx = np.abs(x - CENTER) / RADIUS
shadow_dist = (shadow_dx ** EXPONENT + shadow_dy ** EXPONENT) ** (1.0 / EXPONENT)
shadow_mask = np.clip((1.0 - shadow_dist) * 40.0, 0.0, 1.0)
shadow_arr[:, :, 3] = shadow_mask * 130.0

shadow_img = Image.fromarray(np.uint8(shadow_arr), mode="RGBA").filter(ImageFilter.GaussianBlur(32))

# Composite final image
master_icon = Image.fromarray(np.uint8(np.clip(fc_arr, 0, 255)), mode="RGBA")
final_app_icon = Image.alpha_composite(shadow_img, master_icon)

# Save master 1024x1024 icon
output_path = "/Volumes/SSD.DATA/Leo/APPS/FreeSpaceMacos/app-icon-1024.png"
final_app_icon.save(output_path, "PNG")
print(f"Master app icon created successfully at: {output_path}")
