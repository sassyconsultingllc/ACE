#!/usr/bin/env python3
"""
Sassy Browser — Markdown to PDF Converter
==========================================
Converts all markdown documents in the Business Documents folder
(and subfolders) to well-formatted, encrypted PDFs with proper metadata.

Requirements:
    pip install markdown weasyprint PyPDF2 pymdown-extensions Pillow reportlab

Usage:
    cd "V:\\sassy-browser-FIXED\\Business Documents"
    python convert_to_pdf.py

    # Or convert a single file:
    python convert_to_pdf.py --file "2026-02-14_01-COMPANY-ACTION-PLAN.md"

    # Or a specific folder:
    python convert_to_pdf.py --folder INVEST

All PDFs are encrypted with the configured passkey and include
Sassy Consulting LLC metadata.
"""

import os
import sys
import glob
import argparse
import datetime
import markdown
from pathlib import Path

# ---------------------------------------------------------------------------
# Configuration
# ---------------------------------------------------------------------------
ENCRYPTION_PASSKEY = "7981024"

# Watermark image — centered on every page at low opacity
WATERMARK_IMAGE = Path(__file__).parent / "assets" / "watermark.jpg"
WATERMARK_OPACITY = 0.12  # 12% — visible but doesn't interfere with text
WATERMARK_SCALE = 0.55    # 55% of page width

METADATA = {
    "author":       "Sassy Consulting LLC",
    "creator":      "Sassy Browser Document Generator",
    "producer":     "Sassy Consulting LLC — Confidential",
    "company":      "Sassy Consulting LLC",
    "subject":      "Sassy Browser — Business Documentation",
    "keywords":     "Sassy Browser, Sassy Consulting LLC, Privacy, Browser, Rust",
    "copyright":    "© 2026 Sassy Consulting LLC. All rights reserved.",
    "category":     "Business Documentation",
    "language":     "en-US",
    "confidentiality": "Confidential — Sassy Consulting LLC",
}

# Markdown extensions for rich rendering
MD_EXTENSIONS = [
    "tables",
    "fenced_code",
    "codehilite",
    "toc",
    "attr_list",
    "md_in_html",
    "sane_lists",
    "smarty",
    "meta",
]

MD_EXTENSION_CONFIGS = {
    "codehilite": {
        "css_class": "highlight",
        "linenums": False,
        "guess_lang": True,
    },
    "toc": {
        "permalink": False,
    },
}

# ---------------------------------------------------------------------------
# CSS Stylesheet — controls the PDF appearance
# ---------------------------------------------------------------------------
PDF_STYLESHEET = """
@page {
    size: letter;
    margin: 1in 0.85in 1in 0.85in;

    @top-right {
        content: "Sassy Consulting LLC";
        font-size: 8pt;
        color: #999999;
        font-family: 'Segoe UI', 'Inter', Arial, sans-serif;
    }

    @bottom-center {
        content: counter(page) " of " counter(pages);
        font-size: 8pt;
        color: #999999;
        font-family: 'Segoe UI', 'Inter', Arial, sans-serif;
    }

    @bottom-left {
        content: "Confidential";
        font-size: 7pt;
        color: #cccccc;
        font-family: 'Segoe UI', 'Inter', Arial, sans-serif;
    }
}

@page :first {
    @top-right { content: none; }
    @bottom-left { content: none; }
}

body {
    font-family: 'Segoe UI', 'Inter', 'Helvetica Neue', Arial, sans-serif;
    font-size: 11pt;
    line-height: 1.6;
    color: #1a1a2e;
    max-width: 100%;
}

/* Headings */
h1 {
    font-size: 24pt;
    font-weight: 700;
    color: #1a1b2e;
    border-bottom: 3px solid #00d4aa;
    padding-bottom: 8px;
    margin-top: 0;
    margin-bottom: 16px;
    page-break-after: avoid;
}

h2 {
    font-size: 18pt;
    font-weight: 600;
    color: #1a1b2e;
    border-bottom: 1px solid #e0e0e0;
    padding-bottom: 6px;
    margin-top: 28px;
    margin-bottom: 12px;
    page-break-after: avoid;
}

h3 {
    font-size: 14pt;
    font-weight: 600;
    color: #2d2d4e;
    margin-top: 22px;
    margin-bottom: 8px;
    page-break-after: avoid;
}

h4 {
    font-size: 12pt;
    font-weight: 600;
    color: #3d3d5c;
    margin-top: 18px;
    margin-bottom: 6px;
    page-break-after: avoid;
}

/* Paragraphs */
p {
    margin-bottom: 10px;
    text-align: left;
    orphans: 3;
    widows: 3;
}

/* Bold and italic */
strong { font-weight: 700; }
em { font-style: italic; color: #444466; }

/* Links */
a {
    color: #00a88a;
    text-decoration: none;
}

/* Lists */
ul, ol {
    margin-bottom: 12px;
    padding-left: 24px;
}

li {
    margin-bottom: 4px;
    line-height: 1.5;
}

li > ul, li > ol {
    margin-top: 4px;
    margin-bottom: 4px;
}

/* Checkbox lists */
li input[type="checkbox"] {
    margin-right: 6px;
}

/* Tables */
table {
    width: 100%;
    border-collapse: collapse;
    margin: 16px 0;
    font-size: 10pt;
    page-break-inside: auto;
}

thead {
    background-color: #1a1b2e;
    color: #ffffff;
}

th {
    padding: 10px 12px;
    text-align: left;
    font-weight: 600;
    font-size: 10pt;
    border: 1px solid #1a1b2e;
}

td {
    padding: 8px 12px;
    border: 1px solid #d0d0d0;
    vertical-align: top;
}

tr:nth-child(even) {
    background-color: #f5f7fa;
}

tr {
    page-break-inside: avoid;
}

/* Code blocks */
pre {
    background-color: #f0f2f5;
    border: 1px solid #d0d5dd;
    border-radius: 6px;
    padding: 14px 16px;
    font-family: 'Cascadia Code', 'JetBrains Mono', 'Consolas', 'Courier New', monospace;
    font-size: 9pt;
    line-height: 1.5;
    overflow-wrap: break-word;
    white-space: pre-wrap;
    page-break-inside: avoid;
}

code {
    font-family: 'Cascadia Code', 'JetBrains Mono', 'Consolas', 'Courier New', monospace;
    font-size: 9.5pt;
    background-color: #eef0f4;
    padding: 2px 5px;
    border-radius: 3px;
}

pre code {
    background: none;
    padding: 0;
    border-radius: 0;
}

/* Horizontal rules */
hr {
    border: none;
    border-top: 2px solid #e0e0e0;
    margin: 28px 0;
}

/* Blockquotes */
blockquote {
    border-left: 4px solid #00d4aa;
    margin: 16px 0;
    padding: 10px 20px;
    background-color: #f8fffe;
    color: #2d2d4e;
    font-style: italic;
}

blockquote p {
    margin-bottom: 4px;
}

/* Definition lists */
dt { font-weight: 700; margin-top: 10px; }
dd { margin-left: 20px; margin-bottom: 6px; }

/* Images */
img {
    max-width: 100%;
    height: auto;
}

/* Page break hints */
h1, h2 {
    page-break-before: auto;
}

/* Highlight class for code */
.highlight pre {
    background-color: #f0f2f5;
}

/* Cover page styling */
.cover-page {
    text-align: center;
    padding-top: 200px;
}

.cover-page h1 {
    font-size: 32pt;
    border: none;
    color: #1a1b2e;
}

.cover-page .subtitle {
    font-size: 14pt;
    color: #666;
    margin-top: 20px;
}
"""


def convert_md_to_html(md_content: str, title: str) -> str:
    """Convert markdown content to a full HTML document with styling."""
    md = markdown.Markdown(
        extensions=MD_EXTENSIONS,
        extension_configs=MD_EXTENSION_CONFIGS,
        output_format="html5",
    )
    html_body = md.convert(md_content)

    # Convert markdown checkbox syntax to HTML checkboxes
    html_body = html_body.replace(
        "[ ]", '<input type="checkbox" disabled>'
    ).replace(
        "[x]", '<input type="checkbox" checked disabled>'
    ).replace(
        "[X]", '<input type="checkbox" checked disabled>'
    )

    html_doc = f"""<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="author" content="{METADATA['author']}">
    <meta name="description" content="{METADATA['subject']}">
    <meta name="keywords" content="{METADATA['keywords']}">
    <title>{title}</title>
    <style>
{PDF_STYLESHEET}
    </style>
</head>
<body>
{html_body}
</body>
</html>"""
    return html_doc


def html_to_pdf(html_content: str, output_path: str) -> str:
    """Render HTML to PDF using WeasyPrint."""
    from weasyprint import HTML
    HTML(string=html_content).write_pdf(output_path)
    return output_path


def encrypt_pdf(input_path: str, output_path: str, password: str):
    """Encrypt a PDF with the given password and add metadata."""
    from PyPDF2 import PdfReader, PdfWriter

    reader = PdfReader(input_path)
    writer = PdfWriter()

    for page in reader.pages:
        writer.add_page(page)

    # Add metadata
    writer.add_metadata({
        "/Author":      METADATA["author"],
        "/Creator":     METADATA["creator"],
        "/Producer":    METADATA["producer"],
        "/Title":       Path(output_path).stem,
        "/Subject":     METADATA["subject"],
        "/Keywords":    METADATA["keywords"],
        "/Company":     METADATA["company"],
        "/Copyright":   METADATA["copyright"],
        "/Category":    METADATA["category"],
        "/Language":    METADATA["language"],
        "/CreationDate": datetime.datetime.now().strftime("D:%Y%m%d%H%M%S"),
        "/ModDate":     datetime.datetime.now().strftime("D:%Y%m%d%H%M%S"),
        "/Confidentiality": METADATA["confidentiality"],
    })

    # Encrypt with user password (required to open) and owner password
    # user_password = what you need to type to open the PDF
    # owner_password = controls permissions (printing, copying, etc.)
    writer.encrypt(
        user_password=password,
        owner_password=password,
        use_128bit=True,
    )

    with open(output_path, "wb") as f:
        writer.write(f)


def create_watermark_pdf(output_path: str, page_width: float, page_height: float) -> str:
    """Create a single-page PDF with the watermark image centered at low opacity."""
    from reportlab.lib.units import inch
    from reportlab.pdfgen import canvas
    from PIL import Image
    import io

    if not WATERMARK_IMAGE.exists():
        print(f"  ⚠ Watermark image not found: {WATERMARK_IMAGE}")
        return None

    # Get image dimensions to maintain aspect ratio
    with Image.open(WATERMARK_IMAGE) as img:
        img_w, img_h = img.size
        aspect = img_h / img_w

    # Scale watermark to configured percentage of page width
    wm_width = page_width * WATERMARK_SCALE
    wm_height = wm_width * aspect

    # If height exceeds 40% of page, scale down by height instead
    max_height = page_height * 0.40
    if wm_height > max_height:
        wm_height = max_height
        wm_width = wm_height / aspect

    # Center on page
    x = (page_width - wm_width) / 2
    y = (page_height - wm_height) / 2

    # Create the watermark PDF
    c = canvas.Canvas(output_path, pagesize=(page_width, page_height))

    # Save state, set opacity, draw image, restore
    c.saveState()
    c.setFillAlpha(WATERMARK_OPACITY)
    c.setStrokeAlpha(WATERMARK_OPACITY)

    # Draw the image with transparency
    c.drawImage(
        str(WATERMARK_IMAGE),
        x, y, wm_width, wm_height,
        mask='auto',
        preserveAspectRatio=True,
    )

    c.restoreState()
    c.save()
    return output_path


def apply_watermark(input_path: str, output_path: str):
    """Stamp the watermark image onto every page of a PDF."""
    from PyPDF2 import PdfReader, PdfWriter
    import tempfile

    reader = PdfReader(input_path)
    writer = PdfWriter()

    # Get page dimensions from first page
    first_page = reader.pages[0]
    page_width = float(first_page.mediabox.width)
    page_height = float(first_page.mediabox.height)

    # Create watermark PDF in temp location
    wm_path = tempfile.mktemp(suffix="_watermark.pdf")
    result = create_watermark_pdf(wm_path, page_width, page_height)

    if result is None:
        # No watermark available, just copy input to output
        import shutil
        shutil.copy2(input_path, output_path)
        return

    try:
        wm_reader = PdfReader(wm_path)
        wm_page = wm_reader.pages[0]

        for page in reader.pages:
            # Merge watermark UNDER the content (so text stays on top)
            page.merge_page(wm_page, over=False)
            writer.add_page(page)

        with open(output_path, "wb") as f:
            writer.write(f)
    finally:
        # Clean up temp watermark PDF
        if Path(wm_path).exists():
            Path(wm_path).unlink()


def extract_title_from_md(md_content: str) -> str:
    """Extract the first H1 heading from markdown content."""
    for line in md_content.splitlines():
        stripped = line.strip()
        if stripped.startswith("# "):
            return stripped[2:].strip()
    return "Sassy Browser Document"


def convert_single_file(md_path: str, output_dir: str = None) -> str:
    """Convert a single markdown file to an encrypted PDF."""
    md_path = Path(md_path)
    if not md_path.exists():
        print(f"  ERROR: File not found: {md_path}")
        return None

    # Read markdown content
    with open(md_path, "r", encoding="utf-8") as f:
        md_content = f.read()

    title = extract_title_from_md(md_content)
    pdf_name = md_path.stem + ".pdf"

    # Output directory defaults to same as input
    if output_dir:
        out_dir = Path(output_dir)
    else:
        out_dir = md_path.parent

    out_dir.mkdir(parents=True, exist_ok=True)

    temp_pdf = out_dir / f"_temp_{pdf_name}"
    temp_wm_pdf = out_dir / f"_temp_wm_{pdf_name}"
    final_pdf = out_dir / pdf_name

    try:
        # Step 1: Convert MD → HTML
        html_content = convert_md_to_html(md_content, title)

        # Step 2: HTML → PDF (unencrypted temp)
        html_to_pdf(html_content, str(temp_pdf))

        # Step 3: Apply watermark (centered, 12% opacity, every page)
        apply_watermark(str(temp_pdf), str(temp_wm_pdf))

        # Step 4: Encrypt PDF and add metadata
        encrypt_pdf(str(temp_wm_pdf), str(final_pdf), ENCRYPTION_PASSKEY)

        # Clean up temp files
        for tmp in [temp_pdf, temp_wm_pdf]:
            if tmp.exists():
                tmp.unlink()

        print(f"  ✓ {md_path.name} → {pdf_name} (watermarked + encrypted)")
        return str(final_pdf)

    except Exception as e:
        print(f"  ✗ {md_path.name} — ERROR: {e}")
        # Clean up temp files on error
        for tmp in [temp_pdf, temp_wm_pdf]:
            if tmp.exists():
                tmp.unlink()
        return None


def convert_all(base_dir: str = None):
    """Convert all markdown files in Business Documents and subfolders."""
    if base_dir is None:
        base_dir = Path(__file__).parent

    base_dir = Path(base_dir)
    md_files = sorted(base_dir.rglob("*.md"))

    if not md_files:
        print("No markdown files found.")
        return

    wm_status = "✓ Enabled" if WATERMARK_IMAGE.exists() else "✗ Not found"

    print(f"\n{'='*60}")
    print(f"  Sassy Browser — Markdown to PDF Converter")
    print(f"  Documents: {len(md_files)}")
    print(f"  Watermark: {wm_status} ({WATERMARK_OPACITY*100:.0f}% opacity)")
    print(f"  Encryption: AES-128 (passkey configured)")
    print(f"  Metadata: Sassy Consulting LLC")
    print(f"{'='*60}\n")

    success = 0
    failed = 0

    for md_file in md_files:
        # Skip this script's own file and any non-document MDs
        if md_file.name == "convert_to_pdf.py":
            continue

        result = convert_single_file(str(md_file))
        if result:
            success += 1
        else:
            failed += 1

    print(f"\n{'='*60}")
    print(f"  Results: {success} converted, {failed} failed")
    print(f"  PDFs encrypted with configured passkey")
    print(f"{'='*60}\n")

    # Print metadata summary
    print("PDF Metadata included in all documents:")
    for key, value in METADATA.items():
        print(f"  • {key}: {value}")
    print()
    print("Additional metadata fields per PDF:")
    print("  • /Title        — derived from filename")
    print("  • /CreationDate  — timestamp of conversion")
    print("  • /ModDate       — timestamp of conversion")
    print()


def main():
    parser = argparse.ArgumentParser(
        description="Convert Sassy Browser markdown docs to encrypted PDFs"
    )
    parser.add_argument(
        "--file", "-f",
        help="Convert a single markdown file",
        type=str,
        default=None,
    )
    parser.add_argument(
        "--folder", "-d",
        help="Convert all MDs in a specific subfolder (e.g., INVEST)",
        type=str,
        default=None,
    )
    parser.add_argument(
        "--output", "-o",
        help="Output directory for PDFs (default: same as source)",
        type=str,
        default=None,
    )
    parser.add_argument(
        "--no-encrypt",
        help="Skip encryption (for testing)",
        action="store_true",
        default=False,
    )

    args = parser.parse_args()

    # Check dependencies
    missing = []
    try:
        import markdown
    except ImportError:
        missing.append("markdown")
    try:
        from weasyprint import HTML
    except ImportError:
        missing.append("weasyprint")
    try:
        from PyPDF2 import PdfReader
    except ImportError:
        missing.append("PyPDF2")
    try:
        from PIL import Image
    except ImportError:
        missing.append("Pillow")
    try:
        from reportlab.pdfgen import canvas
    except ImportError:
        missing.append("reportlab")

    if missing:
        print(f"\nMissing dependencies: {', '.join(missing)}")
        print(f"Install with: pip install {' '.join(missing)}")
        if "weasyprint" in missing:
            print("\nWeasyPrint also requires GTK libraries:")
            print("  Windows: Download from https://github.com/nickvdyck/weasyprint-win/releases")
            print("           or install via: pip install weasyprint")
            print("           and follow GTK install instructions at:")
            print("           https://doc.courtbouillon.org/weasyprint/stable/first_steps.html")
            print("  macOS:   brew install pango")
            print("  Linux:   sudo apt install libpango-1.0-0 libpangocairo-1.0-0")
        sys.exit(1)

    # Check watermark image
    if WATERMARK_IMAGE.exists():
        print(f"  Watermark: {WATERMARK_IMAGE} ({WATERMARK_IMAGE.stat().st_size // 1024}KB)")
    else:
        print(f"  ⚠ Watermark not found at: {WATERMARK_IMAGE}")
        print(f"    PDFs will be generated without watermark.")

    if args.file:
        # Single file mode
        print(f"\nConverting single file: {args.file}")
        convert_single_file(args.file, args.output)
    elif args.folder:
        # Folder mode
        base = Path(__file__).parent / args.folder
        if not base.exists():
            print(f"Folder not found: {base}")
            sys.exit(1)
        print(f"\nConverting folder: {base}")
        convert_all(str(base))
    else:
        # Convert everything
        convert_all()


if __name__ == "__main__":
    main()
