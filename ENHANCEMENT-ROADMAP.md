# Sassy Browser - Disruptor Enhancement Roadmap

## Current State Analysis
- **Lines of Code**: ~28,000+ across 80+ Rust files
- **File Types Supported**: 200+ formats
- **Architecture**: Excellent modular viewer design

---

## Priority 1: Complete Edit/Save/Print for All Viewers

### Image Editor (`viewers/image.rs`) - 90% Complete
**Status**: View ✅ | Edit ✅ | Save ✅ | Print ❌
- [x] All raster formats (PNG, JPEG, WebP, TIFF, BMP)
- [x] RAW camera files (CR2, NEF, ARW, DNG)
- [x] Adjustments (brightness, contrast, saturation, hue)
- [x] Filters (grayscale, sepia, blur, sharpen)
- [x] Transforms (rotate, flip, crop, resize)
- [x] Export to multiple formats with quality control
- [x] Undo/redo history
- [ ] **NEEDED**: System print dialog integration
- [ ] **NEEDED**: Layers support (PSD full editing)
- [ ] **NEEDED**: Selection tools (lasso, magic wand)
- [ ] **NEEDED**: Drawing tools (brush, pencil, shapes)

### PDF Viewer (`viewers/pdf.rs`) - 70% Complete
**Status**: View ✅ | Edit ⚠️ | Save ⚠️ | Print ❌
- [x] Text extraction and display
- [x] Page navigation
- [x] Search with highlighting
- [x] Zoom controls
- [ ] **NEEDED**: PDF rendering (images, vectors) - use `pdfium-render` or `mupdf`
- [ ] **NEEDED**: Form filling
- [ ] **NEEDED**: Digital signatures
- [ ] **NEEDED**: Annotation persistence (save to PDF)
- [ ] **NEEDED**: Page manipulation (add, delete, reorder, merge)
- [ ] **NEEDED**: Print dialog

### Document Editor (`viewers/document.rs`) - 75% Complete
**Status**: View ✅ | Edit ✅ | Save ⚠️ | Print ❌
- [x] DOCX/ODT/RTF reading
- [x] Rich text editing
- [x] Formatting (bold, italic, underline, alignment)
- [x] Paragraph styles
- [x] Find/replace
- [x] Undo/redo
- [ ] **NEEDED**: DOCX writing (use `docx-rs` crate)
- [ ] **NEEDED**: Tables
- [ ] **NEEDED**: Images in documents
- [ ] **NEEDED**: Headers/footers
- [ ] **NEEDED**: Page breaks and layout
- [ ] **NEEDED**: Print dialog

### Spreadsheet Editor (`viewers/spreadsheet.rs`) - 60% Complete
**Status**: View ✅ | Edit ⚠️ | Save ⚠️ | Print ❌
- [x] XLSX/XLS/ODS/CSV reading
- [x] Cell display
- [x] Multiple sheets
- [ ] **NEEDED**: Cell editing
- [ ] **NEEDED**: Formula evaluation
- [ ] **NEEDED**: Formatting (fonts, colors, borders)
- [ ] **NEEDED**: Charts
- [ ] **NEEDED**: XLSX writing (use `rust_xlsxwriter`)
- [ ] **NEEDED**: Print dialog

### Chemical Viewer (`viewers/chemical.rs`) - 65% Complete
**Status**: View ✅ | Edit ❌ | Save ❌ | Print ❌
- [x] PDB/MOL/XYZ/CIF parsing
- [x] Atom/bond data structures
- [ ] **NEEDED**: 3D WebGL/wgpu rendering
- [ ] **NEEDED**: Rotation/zoom controls
- [ ] **NEEDED**: Ribbon diagrams (secondary structure)
- [ ] **NEEDED**: Surface rendering
- [ ] **NEEDED**: Distance/angle measurements
- [ ] **NEEDED**: Export to PNG/SVG
- [ ] **NEEDED**: Print diagram

---

## Priority 2: Universal Print System

Create a unified print abstraction that all viewers use:

```rust
// src/print.rs
pub struct PrintJob {
    pub content: PrintContent,
    pub settings: PrintSettings,
}

pub enum PrintContent {
    Image(DynamicImage),
    Pdf(Vec<u8>),
    Document(Vec<PrintPage>),
    Html(String),
}

pub fn print(job: PrintJob) -> Result<()> {
    // Windows: Use PrintDlgW
    // macOS: Use NSPrintOperation
    // Linux: Use CUPS or system dialog
}
```

---

## Priority 3: Missing Format Support

### Images - Add:
- [ ] AVIF (modern compression)
- [ ] HEIC/HEIF (iOS photos)
- [ ] JXL (JPEG XL)

### Documents - Add:
- [ ] Pages (macOS)
- [ ] WPS (Kingsoft)

### Scientific - Add:
- [ ] DICOM (medical imaging)
- [ ] FASTA/FASTQ (genomics)
- [ ] GRO (GROMACS)
- [ ] NetCDF (climate data)

### CAD - Add:
- [ ] DXF/DWG (AutoCAD)
- [ ] STEP/IGES (engineering)

---

## Priority 4: Cross-Format Operations

This is where consolidation DOES make sense - a conversion/export system:

```rust
// src/converter.rs
pub trait Convertible {
    fn to_image(&self) -> Result<DynamicImage>;
    fn to_pdf(&self) -> Result<Vec<u8>>;
    fn to_text(&self) -> Result<String>;
    fn to_html(&self) -> Result<String>;
}

impl Convertible for OpenFile {
    // Dispatch based on file_type
}
```

---

## Recommended Crates to Add

### For PDF Full Rendering:
```toml
pdfium-render = "0.8"  # Google's Pdfium bindings - pixel-perfect rendering
```

### For DOCX Writing:
```toml
docx-rs = "0.4"  # Create/modify DOCX files
```

### For XLSX Writing:
```toml
rust_xlsxwriter = "0.64"  # Excel file creation
```

### For System Print:
```toml
# Windows
windows = { version = "0.52", features = ["Win32_UI_Controls_Dialogs", "Win32_Graphics_Printing"] }

# Cross-platform option
native-dialog = "0.7"  # Already using this
```

### For 3D Rendering (Chemical):
```toml
wgpu = "0.19"  # GPU-accelerated 3D
```

---

## Implementation Order

1. **Week 1-2**: Print system (single implementation, all viewers use it)
2. **Week 3-4**: PDF pixel-perfect rendering with pdfium
3. **Week 5-6**: DOCX/XLSX write support
4. **Week 7-8**: 3D chemical viewer with wgpu
5. **Week 9-10**: Missing format support (HEIC, DICOM, DXF)

---

## Architecture Principle

**Keep viewers separate, but add shared systems:**

```
src/
├── print.rs          ← NEW: Unified print system
├── converter.rs      ← NEW: Format conversion
├── export.rs         ← NEW: Export presets
├── file_handler.rs   ← Universal file detection (already exists)
├── viewers/          ← Keep modular (already optimal)
```

This gives you:
- **Specialization**: Each viewer can match its commercial competitor feature-for-feature
- **Shared Infrastructure**: Print, export, conversion work identically everywhere
- **Compile-time Safety**: Rust ensures all viewers implement required traits
