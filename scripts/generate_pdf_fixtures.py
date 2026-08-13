#!/usr/bin/env python3
"""Gera fixtures PDF sintéticos para testar importação PDF → Markdown.

Conteudo ficticio e neutro, gerado por codigo -- nenhum documento real.

Casos cobertos (do PDF_IMPORT_ACTION_PLAN.md §5):
- simple_headings.pdf: H1, H2, H3 + parágrafos
- table_simple.pdf: tabela 3x4 com headers
- table_merged.pdf: células mescladas (rowspan/colspan)
- two_column.pdf: layout de duas colunas
- code_block.pdf: bloco de código
- image_caption.pdf: figura + legenda
- scanned.pdf: PDF de imagem (negativo — precisa OCR)
"""

from reportlab.lib.pagesizes import A4
from reportlab.lib.styles import getSampleStyleSheet, ParagraphStyle
from reportlab.lib.units import cm
from reportlab.lib import colors
from reportlab.platypus import (
    SimpleDocTemplate, Paragraph, Spacer, Table, TableStyle,
    Frame, PageTemplate, BaseDocTemplate, NextPageTemplate, PageBreak
)
from reportlab.lib.enums import TA_CENTER, TA_LEFT

OUT_DIR = "src-tauri/tests/fixtures"


def gen_simple_headings():
    doc = SimpleDocTemplate(f"{OUT_DIR}/simple_headings.pdf", pagesize=A4)
    styles = getSampleStyleSheet()
    story = [
        Paragraph("Relatório de Vendas", styles["Heading1"]),
        Paragraph("Este documento apresenta os resultados do trimestre.", styles["BodyText"]),
        Spacer(1, 0.3 * cm),
        Paragraph("Primeiro Trimestre", styles["Heading2"]),
        Paragraph("As vendas cresceram 15% no período.", styles["BodyText"]),
        Spacer(1, 0.3 * cm),
        Paragraph("Detalhes Regionais", styles["Heading3"]),
        Paragraph("A região Sul liderou com 40% do total.", styles["BodyText"]),
        Paragraph("Segundo Trimestre", styles["Heading2"]),
        Paragraph("Estabilização com leve alta de 2%.", styles["BodyText"]),
    ]
    doc.build(story)


def gen_table_simple():
    doc = SimpleDocTemplate(f"{OUT_DIR}/table_simple.pdf", pagesize=A4)
    styles = getSampleStyleSheet()
    data = [
        ["Produto", "Qtd", "Preço", "Total"],
        ["Notebook", "10", "2500", "25000"],
        ["Mouse", "50", "80", "4000"],
        ["Teclado", "30", "120", "3600"],
    ]
    t = Table(data, hAlign="LEFT")
    t.setStyle(TableStyle([
        ("BACKGROUND", (0, 0), (-1, 0), colors.grey),
        ("TEXTCOLOR", (0, 0), (-1, 0), colors.whitesmoke),
        ("GRID", (0, 0), (-1, -1), 0.5, colors.black),
        ("FONTSIZE", (0, 0), (-1, -1), 10),
    ]))
    story = [Paragraph("Tabela de Estoque", styles["Heading1"]), Spacer(1, 0.3 * cm), t]
    doc.build(story)


def gen_table_merged():
    doc = SimpleDocTemplate(f"{OUT_DIR}/table_merged.pdf", pagesize=A4)
    styles = getSampleStyleSheet()
    data = [
        ["Categoria", "Item", "Valor"],
        ["A", "X", "100"],
        ["", "Y", "200"],
        ["B", "Z", "150"],
    ]
    t = Table(data, hAlign="LEFT", colWidths=[3 * cm, 3 * cm, 3 * cm])
    t.setStyle(TableStyle([
        ("SPAN", (0, 1), (0, 2)),  # mescla linhas 1-2 col 0
        ("BACKGROUND", (0, 0), (-1, 0), colors.grey),
        ("TEXTCOLOR", (0, 0), (-1, 0), colors.whitesmoke),
        ("GRID", (0, 0), (-1, -1), 0.5, colors.black),
        ("FONTSIZE", (0, 0), (-1, -1), 10),
        ("VALIGN", (0, 0), (-1, -1), "MIDDLE"),
    ]))
    story = [Paragraph("Tabela Mesclada", styles["Heading1"]), Spacer(1, 0.3 * cm), t]
    doc.build(story)


def gen_two_column():
    doc = BaseDocTemplate(f"{OUT_DIR}/two_column.pdf", pagesize=A4)
    frame1 = Frame(2 * cm, 2 * cm, 8 * cm, 25 * cm, id="col1")
    frame2 = Frame(10.5 * cm, 2 * cm, 8 * cm, 25 * cm, id="col2")
    doc.addPageTemplates([PageTemplate(id="two", frames=[frame1, frame2])])
    styles = getSampleStyleSheet()
    story = [
        Paragraph("Coluna Esquerda", styles["Heading2"]),
        Paragraph("Texto da primeira coluna. Lorem ipsum dolor sit amet.", styles["BodyText"]),
        Paragraph("Mais conteúdo à esquerda.", styles["BodyText"]),
        FrameBreak := NextPageTemplate("two"),
        Paragraph("Coluna Direita", styles["Heading2"]),
        Paragraph("Texto da segunda coluna. Consectetur adipiscing elit.", styles["BodyText"]),
        Paragraph("Mais conteúdo à direita.", styles["BodyText"]),
    ]
    doc.build(story)


def gen_code_block():
    from reportlab.lib.styles import ParagraphStyle
    doc = SimpleDocTemplate(f"{OUT_DIR}/code_block.pdf", pagesize=A4)
    styles = getSampleStyleSheet()
    code_style = ParagraphStyle("Code", parent=styles["Code"], fontSize=9, leading=12,
                                 backColor=colors.lightgrey, leftIndent=10)
    code = """def fib(n):
    if n <= 1:
        return n
    return fib(n-1) + fib(n-2)

print(fib(10))"""
    story = [
        Paragraph("Exemplo de Código", styles["Heading1"]),
        Paragraph("Abaixo um exemplo em Python:", styles["BodyText"]),
        Spacer(1, 0.2 * cm),
        Paragraph(code.replace("\n", "<br/>"), code_style),
    ]
    doc.build(story)


def gen_image_caption():
    from reportlab.platypus import Image
    from reportlab.lib.utils import ImageReader
    import io
    from PIL import Image as PILImage
    doc = SimpleDocTemplate(f"{OUT_DIR}/image_caption.pdf", pagesize=A4)
    styles = getSampleStyleSheet()
    # Imagem sintética (quadrado colorido)
    img_buf = io.BytesIO()
    PILImage.new("RGB", (200, 200), (70, 130, 180)).save(img_buf, format="PNG")
    img_buf.seek(0)
    story = [
        Paragraph("Figura de Exemplo", styles["Heading1"]),
        Spacer(1, 0.3 * cm),
        Image(img_buf, width=5 * cm, height=5 * cm),
        Paragraph("Figura 1: Gráfico de barras sintético.", styles["BodyText"]),
    ]
    doc.build(story)


def gen_scanned():
    """PDF de imagem (sem texto selecionável) — caso negativo para OCR."""
    from PIL import Image, ImageDraw
    import io
    from reportlab.platypus import Image as RLImage
    doc = SimpleDocTemplate(f"{OUT_DIR}/scanned.pdf", pagesize=A4)
    styles = getSampleStyleSheet()
    # Renderiza texto como imagem (não selecionável)
    img = Image.new("RGB", (595, 842), "white")
    draw = ImageDraw.Draw(img)
    try:
        from PIL import ImageFont
        font = ImageFont.load_default()
    except Exception:
        font = None
    lines = [
        "Documento Escaneado",
        "",
        "Este e um texto que foi digitalizado",
        "e nao pode ser selecionado como texto.",
        "Requer OCR para extrair conteudo.",
    ]
    y = 100
    for line in lines:
        draw.text((50, y), line, fill="black", font=font)
        y += 30
    buf = io.BytesIO()
    img.save(buf, format="PNG")
    buf.seek(0)
    story = [RLImage(buf, width=15 * cm, height=21 * cm)]
    doc.build(story)


if __name__ == "__main__":
    import os
    os.makedirs(OUT_DIR, exist_ok=True)
    gen_simple_headings()
    gen_table_simple()
    gen_table_merged()
    gen_two_column()
    gen_code_block()
    gen_image_caption()
    gen_scanned()
    print(f"Fixtures gerados em {OUT_DIR}/")
