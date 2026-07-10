#!/usr/bin/env python3
"""Gera o icone-fonte (1024x1024) do Markforge usado pelo `tauri icon` em CI."""
from PIL import Image, ImageDraw

SIZE = 1024
BG = (24, 42, 68, 255)        # navy escuro
BG2 = (31, 58, 95, 255)       # navy "ACCENT"
SPARK = (232, 163, 61, 255)   # ambar (faisca da forja)
SPARK2 = (245, 196, 120, 255)

img = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
d = ImageDraw.Draw(img)

# fundo: quadrado arredondado com leve gradiente vertical simulado em faixas
radius = 190
d.rounded_rectangle([0, 0, SIZE, SIZE], radius=radius, fill=BG)
steps = 40
for i in range(steps):
    t = i / steps
    y0 = int(SIZE * t)
    y1 = int(SIZE * (t + 1 / steps)) + 1
    r = int(BG[0] + (BG2[0] - BG[0]) * t)
    g = int(BG[1] + (BG2[1] - BG[1]) * t)
    b = int(BG[2] + (BG2[2] - BG[2]) * t)
    band = Image.new("RGBA", (SIZE, y1 - y0), (r, g, b, 255))
    img.paste(band, (0, y0))
mask = Image.new("L", (SIZE, SIZE), 0)
ImageDraw.Draw(mask).rounded_rectangle([0, 0, SIZE, SIZE], radius=radius, fill=255)
bg_final = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
bg_final.paste(img, (0, 0), mask)
img = bg_final
d = ImageDraw.Draw(img)

# marca: "M" geometrico feito de bigornas/triangulos, estilo forja
cx, cy = SIZE / 2, SIZE / 2 + 20
w = 520
h = 380
x0 = cx - w / 2
x1 = cx + w / 2
top = cy - h / 2
bot = cy + h / 2
mid = cy - h / 2 + h * 0.42

stroke = 78
pts_left = [
    (x0, bot), (x0, top), (x0 + stroke * 1.6, top),
    (cx, mid), (cx, top + h * 0.16)
]
# Desenha o "M" como uma polyline espessa usando linhas conectadas
def thick_polyline(points, width, fill):
    d.line(points, fill=fill, width=width, joint="curve")
    r = width / 2
    for p in points:
        d.ellipse([p[0]-r, p[1]-r, p[0]+r, p[1]+r], fill=fill)

path = [
    (x0 + stroke/2, bot),
    (x0 + stroke/2, top + stroke/2),
    (cx, mid + 20),
    (x1 - stroke/2, top + stroke/2),
    (x1 - stroke/2, bot),
]
thick_polyline(path, stroke, (245, 247, 250, 255))

# faisca / accent no vertice central do M
spark_r = 34
d.ellipse([cx - spark_r, mid - spark_r + 20, cx + spark_r, mid + spark_r + 20], fill=SPARK)
d.ellipse([cx - spark_r*0.5, mid - spark_r*0.5 + 20, cx + spark_r*0.5, mid + spark_r*0.5 + 20], fill=SPARK2)

img.save("app-icon.png")
print("OK: app-icon.png (1024x1024) gerado")
