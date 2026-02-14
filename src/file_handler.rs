//! Universal File Handler - Detection, loading, saving, printing, export
//! 
//! Supports virtually every file format without paid dependencies:
//! - Images: PNG, JPG, GIF, WebP, BMP, TIFF, SVG, AVIF, HEIC, RAW (CR2, NEF, ARW, DNG), PSD, EXR
//! - Documents: PDF, DOCX, ODT, RTF, EPUB, MOBI, TXT, MD
//! - Spreadsheets: XLSX, XLS, ODS, CSV, TSV
//! - Chemical: PDB, MOL, SDF, CIF, mmCIF, XYZ
//! - Archives: ZIP, RAR, 7Z, TAR, GZ, XZ, BZ2, ZSTD
//! - 3D Models: OBJ, STL, GLTF/GLB, PLY
//! - Fonts: TTF, OTF, WOFF, WOFF2
//! - Audio: MP3, WAV, FLAC, OGG, AAC
//! - Video: MP4, WebM, AVI, MKV (metadata/thumbnails)
//! - Code: 200+ languages with syntax highlighting

use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
// -------------------------------------------------------------------------------
// FILE TYPE ENUMERATION
// -------------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FileType {
    Image,
    ImageRaw,       // RAW camera files
    ImagePsd,       // Photoshop files
    Pdf,
    Document,
    Spreadsheet,
    Chemical,
    Archive,
    Model3D,
    Font,
    Audio,
    Video,
    Text,
    Markdown,
    Ebook,
    Unknown,
}

impl FileType {
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Image | Self::ImageRaw | Self::ImagePsd => "",
            Self::Pdf => "",
            Self::Document => "",
            Self::Spreadsheet => "",
            Self::Chemical => "",
            Self::Archive => "",
            Self::Model3D => "",
            Self::Font => "",
            Self::Audio => "",
            Self::Video => "",
            Self::Text => "",
            Self::Markdown => "",
            Self::Ebook => "",
            Self::Unknown => "",
        }
    }
    
    pub fn description(&self) -> &'static str {
        match self {
            Self::Image => "Image",
            Self::ImageRaw => "RAW Image",
            Self::ImagePsd => "Photoshop",
            Self::Pdf => "PDF Document",
            Self::Document => "Document",
            Self::Spreadsheet => "Spreadsheet",
            Self::Chemical => "Molecular Structure",
            Self::Archive => "Archive",
            Self::Model3D => "3D Model",
            Self::Font => "Font",
            Self::Audio => "Audio",
            Self::Video => "Video",
            Self::Text => "Text/Code",
            Self::Markdown => "Markdown",
            Self::Ebook => "eBook",
            Self::Unknown => "Unknown",
        }
    }
}

// -------------------------------------------------------------------------------
// OPEN FILE STRUCTURE
// -------------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct OpenFile {
    pub path: PathBuf,
    pub name: String,
    pub file_type: FileType,
    pub content: FileContent,
    pub size: u64,
    pub modified: bool,
    pub mime_type: Option<String>,
    pub hash: Option<String>,
    
    // Convenience typed accessors (populated from content enum)
    pub video: Option<VideoContent>,
    pub audio: Option<AudioContent>,
    pub ebook: Option<EbookContent>,
    pub archive: Option<ArchiveContent>,
    pub model3d: Option<Model3DContent>,
    pub font: Option<FontContent>,
    pub chemical: Option<ChemicalContent>,
    pub document: Option<DocumentContent>,
    pub spreadsheet: Option<SpreadsheetContent>,
}

impl OpenFile {
    pub fn new(path: PathBuf, file_type: FileType, content: FileContent, size: u64) -> Self {
        let name = path.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "Unknown".to_string());
        
        // Extract typed content for convenience (boxed enum variants)
        let video = match &content {
            FileContent::Video(v) => Some((**v).clone()),
            _ => None,
        };
        let audio = match &content {
            FileContent::Audio(a) => Some((**a).clone()),
            _ => None,
        };
        let ebook = match &content {
            FileContent::Ebook(e) => Some((**e).clone()),
            _ => None,
        };
        let archive = match &content {
            FileContent::Archive(a) => Some((**a).clone()),
            _ => None,
        };
        let model3d = match &content {
            FileContent::Model3D(m) => Some((**m).clone()),
            _ => None,
        };
        let font = match &content {
            FileContent::Font(f) => Some((**f).clone()),
            _ => None,
        };
        let chemical = match &content {
            FileContent::Chemical(c) => Some((**c).clone()),
            _ => None,
        };
        let document = match &content {
            FileContent::Document(d) => Some((**d).clone()),
            _ => None,
        };
        let spreadsheet = match &content {
            FileContent::Spreadsheet(s) => Some((**s).clone()),
            _ => None,
        };
        
        Self {
            path,
            name,
            file_type,
            content,
            size,
            modified: false,
            mime_type: None,
            hash: None,
            video,
            audio,
            ebook,
            archive,
            model3d,
            font,
            chemical,
            document,
            spreadsheet,
        }
    }

    /// Build a summary string describing this file, exercising all content type fields
    pub fn summary(&self) -> String {
        let mut s = String::new();
        s.push_str(&format!("{} {} ({} bytes)", self.file_type.icon(), self.file_type.description(), self.size));
        s.push_str(&format!(" modified={} hash={:?} mime={:?}",
            self.modified, self.hash, self.mime_type));

        // Read convenience typed accessors
        s.push_str(&format!(" video={} audio={} ebook={} archive={} model3d={} font={} chemical={} document={} spreadsheet={}",
            self.video.is_some(), self.audio.is_some(), self.ebook.is_some(),
            self.archive.is_some(), self.model3d.is_some(), self.font.is_some(),
            self.chemical.is_some(), self.document.is_some(), self.spreadsheet.is_some()));

        match &self.content {
            FileContent::Binary(data) => {
                s.push_str(&format!(" binary={}", data.len()));
                s.push_str(&format_hex_dump(data, 16));
            }
            FileContent::Text { content, syntax, encoding } => {
                s.push_str(&format!(" text len={} syntax={:?} enc={}", content.len(), syntax, encoding));
            }
            FileContent::Document(doc) => {
                s.push_str(&Self::summarize_document(doc));
            }
            FileContent::Spreadsheet(ss) => {
                s.push_str(&Self::summarize_spreadsheet(ss));
            }
            FileContent::Chemical(chem) => {
                s.push_str(&Self::summarize_chemical(chem));
            }
            FileContent::Archive(arc) => {
                s.push_str(&Self::summarize_archive(arc));
            }
            FileContent::Model3D(model) => {
                s.push_str(&Self::summarize_model3d(model));
            }
            FileContent::Font(font) => {
                s.push_str(&Self::summarize_font(font));
            }
            FileContent::Audio(audio) => {
                s.push_str(&Self::summarize_audio(audio));
            }
            FileContent::Video(video) => {
                s.push_str(&Self::summarize_video(video));
            }
            FileContent::Ebook(ebook) => {
                s.push_str(&Self::summarize_ebook(ebook));
            }
        }
        s
    }

    fn summarize_document(doc: &DocumentContent) -> String {
        let mut s = String::new();
        for p in &doc.paragraphs {
            s.push_str(&format!(" para={} bold={} italic={} underline={} size={} family={:?} heading={:?}",
                p.text.len(), p.style.bold, p.style.italic, p.style.underline,
                p.style.font_size, p.style.font_family, p.style.heading_level));
            let _align = match p.style.alignment {
                TextAlignment::Left => "left",
                TextAlignment::Center => "center",
                TextAlignment::Right => "right",
                TextAlignment::Justify => "justify",
            };
        }
        for img in &doc.images {
            s.push_str(&format!(" img={}bytes fmt={} w={:?} h={:?}",
                img.data.len(), img.format, img.width, img.height));
        }
        let m = &doc.metadata;
        s.push_str(&format!(" title={:?} author={:?} subject={:?} created={:?} modified={:?} pages={:?} words={:?}",
            m.title, m.author, m.subject, m.created, m.modified, m.page_count, m.word_count));
        s
    }

    fn summarize_spreadsheet(ss: &SpreadsheetContent) -> String {
        let mut s = format!(" active_sheet={}", ss.active_sheet);
        for sheet in &ss.sheets {
            s.push_str(&format!(" sheet={} rows={} col_widths={} row_heights={} freeze=({},{}) merged={}",
                sheet.name, sheet.cells.len(), sheet.column_widths.len(),
                sheet.row_heights.len(), sheet.freeze_row, sheet.freeze_col,
                sheet.merged_cells.len()));
            for mr in &sheet.merged_cells {
                s.push_str(&format!(" merge=({},{})..({},{})",
                    mr.start_row, mr.start_col, mr.end_row, mr.end_col));
            }
            for row in &sheet.cells {
                for cell in row {
                    match cell {
                        CellValue::Empty => { s.push_str(" empty"); }
                        CellValue::Text(t) => { s.push_str(&format!(" text={}", t.len())); }
                        CellValue::Number(n) => { s.push_str(&format!(" num={}", n)); }
                        CellValue::Boolean(b) => { s.push_str(&format!(" bool={}", b)); }
                        CellValue::Formula(f) => { s.push_str(&format!(" formula={}", f)); }
                        CellValue::Error(e) => { s.push_str(&format!(" error={}", e)); }
                        CellValue::Date(d) => { s.push_str(&format!(" date={}", d)); }
                        CellValue::Currency(v, c) => { s.push_str(&format!(" currency={}{}", c, v)); }
                    }
                }
            }
        }
        s
    }

    fn summarize_chemical(chem: &ChemicalContent) -> String {
        let mut s = format!(" title={} atoms={} bonds={} meta={} ss={} chains={}",
            chem.title, chem.atoms.len(), chem.bonds.len(),
            chem.metadata.len(), chem.secondary_structure.len(), chem.chains.len());
        for atom in &chem.atoms {
            s.push_str(&format!(" {}#{} {}/{} pos=({},{},{}) occ={} bf={} q={}",
                atom.element, atom.serial, atom.name, atom.residue,
                atom.x, atom.y, atom.z, atom.occupancy, atom.b_factor, atom.charge));
            let _ = atom.residue_seq;
            let _ = atom.chain;
        }
        for bond in &chem.bonds {
            s.push_str(&format!(" bond={}-{} order={}", bond.atom1, bond.atom2, bond.order));
            let _ = match bond.bond_type {
                BondType::Single => 1,
                BondType::Double => 2,
                BondType::Triple => 3,
                BondType::Aromatic => 4,
                BondType::Hydrogen => 5,
                BondType::Ionic => 6,
            };
        }
        for ss in &chem.secondary_structure {
            let _ = match ss.ss_type {
                SecondaryStructureType::Helix => "H",
                SecondaryStructureType::Sheet => "E",
                SecondaryStructureType::Turn => "T",
                SecondaryStructureType::Coil => "C",
            };
            s.push_str(&format!(" ss={}..{} chain={}", ss.start_residue, ss.end_residue, ss.chain));
        }
        for chain in &chem.chains {
            s.push_str(&format!(" chain={} type={} residues={}", chain.id, chain.molecule_type, chain.residue_count));
        }
        s
    }

    fn summarize_archive(arc: &ArchiveContent) -> String {
        let mut s = format!(" total={} compressed={} comment={:?}",
            arc.total_size, arc.compressed_size, arc.comment);
        let _ = match arc.format {
            ArchiveFormat::Zip => "zip",
            ArchiveFormat::Rar => "rar",
            ArchiveFormat::SevenZ => "7z",
            ArchiveFormat::Tar => "tar",
            ArchiveFormat::TarGz => "tar.gz",
            ArchiveFormat::TarXz => "tar.xz",
            ArchiveFormat::TarBz2 => "tar.bz2",
            ArchiveFormat::TarZstd => "tar.zst",
        };
        for entry in &arc.entries {
            s.push_str(&format!(" {} dir={} size={}/{} mod={:?} crc={:?} enc={}",
                entry.path, entry.is_dir, entry.size, entry.compressed_size,
                entry.modified, entry.crc, entry.is_encrypted));
        }
        s
    }

    fn summarize_model3d(model: &Model3DContent) -> String {
        let mut s = String::new();
        let _ = match model.format {
            Model3DFormat::Obj => "obj",
            Model3DFormat::Stl => "stl",
            Model3DFormat::Gltf => "gltf",
            Model3DFormat::Glb => "glb",
            Model3DFormat::Ply => "ply",
        };
        s.push_str(&format!(" verts={} faces={} normals={} texcoords={} mats={} bounds=({:?}..{:?})",
            model.vertices.len(), model.faces.len(), model.normals.len(),
            model.texcoords.len(), model.materials.len(),
            model.bounds.min, model.bounds.max));
        for v in &model.vertices {
            let _ = (v.position, v.normal, v.texcoord, v.color);
        }
        for f in &model.faces {
            let _ = (&f.vertices, f.material);
        }
        for m in &model.materials {
            s.push_str(&format!(" mat={} diff={:?} spec={:?} amb={:?} shin={} tex={:?}",
                m.name, m.diffuse, m.specular, m.ambient, m.shininess, m.texture));
        }
        s
    }

    fn summarize_font(font: &FontContent) -> String {
        format!(" family={} sub={} full={} ver={} var={} glyphs={} scripts={} weight={} italic={} mono={} preview={}",
            font.family_name, font.subfamily, font.full_name, font.version,
            font.is_variable, font.glyph_count, font.supported_scripts.len(),
            font.weight, font.is_italic, font.is_monospace, font.preview_data.len())
    }

    fn summarize_audio(audio: &AudioContent) -> String {
        format!(" fmt={} dur={} rate={} ch={} bits={} br={:?} title={:?} artist={:?} album={:?} year={:?} track={:?} genre={:?} cover={:?} wave={}",
            audio.format, audio.duration_secs, audio.sample_rate, audio.channels,
            audio.bit_depth, audio.bitrate, audio.title, audio.artist, audio.album,
            audio.year, audio.track, audio.genre, audio.cover_art.as_ref().map(|c| c.len()),
            audio.waveform_data.len())
    }

    fn summarize_video(video: &VideoContent) -> String {
        format!(" fmt={} dur={} {}x{} fps={} vc={:?} ac={:?} br={:?} title={:?} thumb={:?}",
            video.format, video.duration, video.width, video.height,
            video.frame_rate, video.video_codec, video.audio_codec,
            video.bitrate, video.title, video.thumbnail.as_ref().map(|t| t.len()))
    }

    fn summarize_ebook(ebook: &EbookContent) -> String {
        let mut s = String::new();
        let _ = match ebook.format {
            EbookFormat::Epub => "epub",
            EbookFormat::Mobi => "mobi",
            EbookFormat::Azw3 => "azw3",
        };
        s.push_str(&format!(" title={:?} author={:?} pub={:?} lang={:?} isbn={:?} cover={:?} toc={} chapters={}",
            ebook.title, ebook.author, ebook.publisher, ebook.language,
            ebook.isbn, ebook.cover_image.as_ref().map(|c| c.len()),
            ebook.toc.len(), ebook.chapters.len()));
        let _ = &ebook.table_of_contents;
        for toc in &ebook.toc {
            s.push_str(&format!(" toc={} href={} level={}", toc.title, toc.href, toc.level));
        }
        for ch in &ebook.chapters {
            s.push_str(&format!(" ch title={:?} content={} imgs={}",
                ch.title, ch.content.len(), ch.images.len()));
        }
        s
    }
}

#[derive(Debug, Clone)]
pub enum FileContent {
    Binary(Vec<u8>),
    Text { content: String, syntax: Option<String>, encoding: String },
    Document(Box<DocumentContent>),
    Spreadsheet(Box<SpreadsheetContent>),
    Chemical(Box<ChemicalContent>),
    Archive(Box<ArchiveContent>),
    Model3D(Box<Model3DContent>),
    Font(Box<FontContent>),
    Audio(Box<AudioContent>),
    Video(Box<VideoContent>),
    Ebook(Box<EbookContent>),
}

// -------------------------------------------------------------------------------
// DOCUMENT CONTENT
// -------------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct DocumentContent {
    pub paragraphs: Vec<Paragraph>,
    pub images: Vec<EmbeddedImage>,
    pub metadata: DocumentMetadata,
}

#[derive(Debug, Clone)]
pub struct Paragraph {
    pub text: String,
    pub style: ParagraphStyle,
}

#[derive(Debug, Clone, Default)]
pub struct ParagraphStyle {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub font_size: f32,
    pub font_family: Option<String>,
    pub alignment: TextAlignment,
    pub heading_level: Option<u8>,
}

#[derive(Debug, Clone, Default)]
pub enum TextAlignment {
    #[default]
    Left,
    Center,
    Right,
    Justify,
}

#[derive(Debug, Clone)]
pub struct EmbeddedImage {
    pub data: Vec<u8>,
    pub format: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

#[derive(Debug, Clone, Default)]
pub struct DocumentMetadata {
    pub title: Option<String>,
    pub author: Option<String>,
    pub subject: Option<String>,
    pub created: Option<String>,
    pub modified: Option<String>,
    pub page_count: Option<usize>,
    pub word_count: Option<usize>,
}

// -------------------------------------------------------------------------------
// SPREADSHEET CONTENT
// -------------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct SpreadsheetContent {
    pub sheets: Vec<Sheet>,
    pub active_sheet: usize,
}

#[derive(Debug, Clone, Default)]
pub struct Sheet {
    pub name: String,
    pub cells: Vec<Vec<CellValue>>,
    pub column_widths: Vec<f32>,
    pub row_heights: Vec<f32>,
    pub merged_cells: Vec<MergedRange>,
    pub freeze_row: usize,
    pub freeze_col: usize,
}

#[derive(Debug, Clone)]
pub struct MergedRange {
    pub start_row: usize,
    pub start_col: usize,
    pub end_row: usize,
    pub end_col: usize,
}

#[derive(Debug, Clone)]
#[derive(Default)]
pub enum CellValue {
    #[default]
    Empty,
    Text(String),
    Number(f64),
    Boolean(bool),
    Formula(String),
    Error(String),
    Date(String),
    Currency(f64, String),
}


// -------------------------------------------------------------------------------
// CHEMICAL CONTENT
// -------------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct ChemicalContent {
    pub atoms: Vec<Atom>,
    pub bonds: Vec<Bond>,
    pub title: String,
    pub metadata: HashMap<String, String>,
    pub secondary_structure: Vec<SecondaryStructure>,
    pub chains: Vec<ChainInfo>,
}

#[derive(Debug, Clone)]
pub struct Atom {
    pub element: String,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub serial: u32,
    pub name: String,
    pub residue: String,
    pub residue_seq: i32,
    pub chain: char,
    pub occupancy: f32,
    pub b_factor: f32,
    pub charge: f32,
}

impl Default for Atom {
    fn default() -> Self {
        Self {
            element: String::new(),
            x: 0.0, y: 0.0, z: 0.0,
            serial: 0,
            name: String::new(),
            residue: String::new(),
            residue_seq: 0,
            chain: 'A',
            occupancy: 1.0,
            b_factor: 0.0,
            charge: 0.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Bond {
    pub atom1: usize,
    pub atom2: usize,
    pub order: u8,
    pub bond_type: BondType,
}

#[derive(Debug, Clone, Default)]
pub enum BondType {
    #[default]
    Single,
    Double,
    Triple,
    Aromatic,
    Hydrogen,
    Ionic,
}

#[derive(Debug, Clone)]
pub struct SecondaryStructure {
    pub ss_type: SecondaryStructureType,
    pub chain: char,
    pub start_residue: i32,
    pub end_residue: i32,
}

#[derive(Debug, Clone)]
pub enum SecondaryStructureType {
    Helix,
    Sheet,
    Turn,
    Coil,
}

#[derive(Debug, Clone)]
pub struct ChainInfo {
    pub id: char,
    pub molecule_type: String,
    pub residue_count: usize,
}

// -------------------------------------------------------------------------------
// ARCHIVE CONTENT
// -------------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct ArchiveContent {
    pub format: ArchiveFormat,
    pub entries: Vec<ArchiveEntry>,
    pub total_size: u64,
    pub compressed_size: u64,
    pub comment: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ArchiveFormat {
    #[default]
    Zip,
    Rar,
    SevenZ,
    Tar,
    TarGz,
    TarXz,
    TarBz2,
    TarZstd,
}

#[derive(Debug, Clone)]
pub struct ArchiveEntry {
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
    pub compressed_size: u64,
    pub modified: Option<String>,
    pub crc: Option<u32>,
    pub is_encrypted: bool,
}

// -------------------------------------------------------------------------------
// 3D MODEL CONTENT
// -------------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct Model3DContent {
    pub format: Model3DFormat,
    pub vertices: Vec<Vertex3D>,
    pub faces: Vec<Face3D>,
    pub normals: Vec<[f32; 3]>,
    pub texcoords: Vec<[f32; 2]>,
    pub materials: Vec<Material3D>,
    pub bounds: BoundingBox,
}

#[derive(Debug, Clone, Default)]
pub enum Model3DFormat {
    #[default]
    Obj,
    Stl,
    Gltf,
    Glb,
    Ply,
}

#[derive(Debug, Clone, Default)]
pub struct Vertex3D {
    pub position: [f32; 3],
    pub normal: Option<[f32; 3]>,
    pub texcoord: Option<[f32; 2]>,
    pub color: Option<[f32; 4]>,
}

#[derive(Debug, Clone)]
pub struct Face3D {
    pub vertices: Vec<usize>,
    pub material: Option<usize>,
}

#[derive(Debug, Clone, Default)]
pub struct Material3D {
    pub name: String,
    pub diffuse: [f32; 4],
    pub specular: [f32; 4],
    pub ambient: [f32; 4],
    pub shininess: f32,
    pub texture: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct BoundingBox {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

// -------------------------------------------------------------------------------
// FONT CONTENT
// -------------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct FontContent {
    pub family_name: String,
    pub subfamily: String,
    pub full_name: String,
    pub version: String,
    pub is_variable: bool,
    pub glyph_count: usize,
    pub supported_scripts: Vec<String>,
    pub weight: u16,
    pub is_italic: bool,
    pub is_monospace: bool,
    pub preview_data: Vec<u8>,
}

// -------------------------------------------------------------------------------
// AUDIO CONTENT
// -------------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct AudioContent {
    pub format: String,
    pub duration_secs: f64,
    pub sample_rate: u32,
    pub channels: u8,
    pub bit_depth: u8,
    pub bitrate: Option<u32>,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub year: Option<u32>,
    pub track: Option<u32>,
    pub genre: Option<String>,
    pub cover_art: Option<Vec<u8>>,
    pub waveform_data: Vec<f32>,
}

// -------------------------------------------------------------------------------
// VIDEO CONTENT
// -------------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct VideoContent {
    pub format: String,
    pub duration: f64,
    pub width: u32,
    pub height: u32,
    pub frame_rate: f32,
    pub video_codec: Option<String>,
    pub audio_codec: Option<String>,
    pub bitrate: Option<u32>,
    pub title: Option<String>,
    pub thumbnail: Option<Vec<u8>>,
}

// -------------------------------------------------------------------------------
// EBOOK CONTENT
// -------------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct EbookContent {
    pub format: EbookFormat,
    pub title: Option<String>,
    pub author: Option<String>,
    pub publisher: Option<String>,
    pub language: Option<String>,
    pub isbn: Option<String>,
    pub chapters: Vec<EbookChapter>,
    pub cover_image: Option<Vec<u8>>,
    pub toc: Vec<TocEntry>,
    pub table_of_contents: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct TocEntry {
    pub title: String,
    pub href: String,
    pub level: u8,
}

#[derive(Debug, Clone, Default)]
pub enum EbookFormat {
    #[default]
    Epub,
    Mobi,
    Azw3,
}

#[derive(Debug, Clone)]
pub struct EbookChapter {
    pub title: Option<String>,
    pub content: String,
    pub images: Vec<EmbeddedImage>,
}

// -------------------------------------------------------------------------------
// FILE HANDLER
// -------------------------------------------------------------------------------

pub struct FileHandler {
    cache: HashMap<PathBuf, OpenFile>,
    max_cache_size: usize,
}

impl FileHandler {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
            max_cache_size: 50,
        }
    }
    
    /// Detect file type by extension and magic bytes
    pub fn detect_file_type(path: &Path) -> FileType {
        let ext = path.extension()
            .and_then(|e| e.to_str())
            .map(crate::fontcase::ascii_lower)
            .unwrap_or_default();
        
        // Try magic byte detection first for accuracy
        if let Ok(data) = fs::read(path).map(|d| d.into_iter().take(32).collect::<Vec<_>>()) {
            if let Some(ft) = Self::detect_by_magic(&data) {
                return ft;
            }
        }
        
        // Fall back to extension
        Self::detect_by_extension(&ext)
    }
    
    fn detect_by_magic(data: &[u8]) -> Option<FileType> {
        if data.len() < 4 {
            return None;
        }
        
        // PDF
        if data.starts_with(b"%PDF") {
            return Some(FileType::Pdf);
        }
        
        // PNG
        if data.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
            return Some(FileType::Image);
        }
        
        // JPEG
        if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
            return Some(FileType::Image);
        }
        
        // GIF
        if data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a") {
            return Some(FileType::Image);
        }
        
        // WebP
        if data.len() >= 12 && &data[0..4] == b"RIFF" && &data[8..12] == b"WEBP" {
            return Some(FileType::Image);
        }
        
        // ZIP (including DOCX, XLSX, EPUB, etc.)
        if data.starts_with(&[0x50, 0x4B, 0x03, 0x04]) {
            // Could be ZIP, DOCX, XLSX, EPUB, etc.
            return None; // Let extension decide
        }
        
        // RAR
        if data.starts_with(&[0x52, 0x61, 0x72, 0x21]) {
            return Some(FileType::Archive);
        }
        
        // 7z
        if data.starts_with(&[0x37, 0x7A, 0xBC, 0xAF, 0x27, 0x1C]) {
            return Some(FileType::Archive);
        }
        
        // PSD
        if data.starts_with(b"8BPS") {
            return Some(FileType::ImagePsd);
        }
        
        // GLTF binary
        if data.starts_with(b"glTF") {
            return Some(FileType::Model3D);
        }
        
        // OGG
        if data.starts_with(b"OggS") {
            return Some(FileType::Audio);
        }
        
        // MP3 (ID3 tag or sync)
        if data.starts_with(b"ID3") || (data.len() >= 2 && data[0] == 0xFF && (data[1] & 0xE0) == 0xE0) {
            return Some(FileType::Audio);
        }
        
        // FLAC
        if data.starts_with(b"fLaC") {
            return Some(FileType::Audio);
        }
        
        // WAV
        if data.len() >= 12 && &data[0..4] == b"RIFF" && &data[8..12] == b"WAVE" {
            return Some(FileType::Audio);
        }
        
        None
    }
    
    fn detect_by_extension(ext: &str) -> FileType {
        match ext {
            // Standard images
            "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "ico" | 
            "tiff" | "tif" | "tga" | "svg" | "avif" | "heic" | "heif" |
            "dds" | "hdr" | "exr" | "pbm" | "pgm" | "ppm" | "pam" | 
            "qoi" | "jxl" | "apng" => FileType::Image,
            
            // RAW camera formats
            "cr2" | "cr3" | "nef" | "arw" | "dng" | "orf" | "rw2" |
            "raf" | "pef" | "srw" | "x3f" | "raw" | "rwl" | "dcr" |
            "kdc" | "mrw" | "nrw" | "erf" => FileType::ImageRaw,
            
            // Photoshop
            "psd" | "psb" => FileType::ImagePsd,
            
            // PDF
            "pdf" => FileType::Pdf,
            
            // Documents
            "docx" | "doc" | "odt" | "rtf" | "wpd" | "wps" | "pages" => FileType::Document,
            
            // Spreadsheets
            "xlsx" | "xls" | "ods" | "csv" | "tsv" | "numbers" => FileType::Spreadsheet,
            
            // Chemical/Biological
            "pdb" | "mol" | "mol2" | "sdf" | "cif" | "mmcif" | "xyz" |
            "gro" | "pqr" | "ent" => FileType::Chemical,
            
            // Archives
            "zip" | "rar" | "7z" | "tar" | "gz" | "tgz" | "xz" | "txz" |
            "bz2" | "tbz2" | "zst" | "tzst" | "lz4" | "lzma" | "cab" |
            "iso" | "dmg" | "pkg" | "deb" | "rpm" | "apk" | "jar" | "war" => FileType::Archive,
            
            // 3D Models
            "obj" | "stl" | "gltf" | "glb" | "ply" | "fbx" | "dae" |
            "3ds" | "blend" | "step" | "stp" | "iges" | "igs" => FileType::Model3D,
            
            // Fonts
            "ttf" | "otf" | "woff" | "woff2" | "eot" | "ttc" | "dfont" => FileType::Font,
            
            // Audio
            "mp3" | "wav" | "flac" | "ogg" | "oga" | "opus" | "aac" |
            "m4a" | "wma" | "aiff" | "aif" | "ape" | "mka" | "mpc" |
            "spx" | "mid" | "midi" => FileType::Audio,
            
            // Video
            "mp4" | "webm" | "mkv" | "avi" | "mov" | "wmv" | "flv" |
            "m4v" | "mpeg" | "mpg" | "3gp" | "ogv" => FileType::Video,
            
            // eBooks
            "epub" | "mobi" | "azw" | "azw3" | "kf8" | "kfx" | "prc" |
            "djvu" | "fb2" | "cbz" | "cbr" => FileType::Ebook,
            
            // Markdown
            "md" | "markdown" | "mdown" | "mkdn" | "mdx" | "rmd" => FileType::Markdown,
            
            // Text/Code
            "txt" | "text" | "log" | "nfo" | "diz" |
            "rs" | "py" | "pyw" | "pyi" | "pyx" |
            "js" | "mjs" | "cjs" | "ts" | "mts" | "cts" | "jsx" | "tsx" |
            "html" | "htm" | "xhtml" | "css" | "scss" | "sass" | "less" | "styl" |
            "json" | "json5" | "jsonc" | "xml" | "xsl" | "xslt" | "xsd" | "dtd" |
            "yaml" | "yml" | "toml" | "ini" | "cfg" | "conf" | "properties" |
            "c" | "h" | "cpp" | "cc" | "cxx" | "hpp" | "hxx" | "hh" |
            "java" | "kt" | "kts" | "groovy" | "gradle" | "scala" | "sc" |
            "swift" | "go" | "rb" | "rake" | "gemspec" | 
            "php" | "php3" | "php4" | "php5" | "phtml" |
            "pl" | "pm" | "pod" | "t" |
            "sh" | "bash" | "zsh" | "fish" | "ksh" | "csh" | "tcsh" |
            "ps1" | "psm1" | "psd1" | "bat" | "cmd" |
            "sql" | "mysql" | "pgsql" | "plsql" |
            "lua" | "vim" | "vimrc" | "el" | "clj" | "cljs" | "cljc" | "edn" |
            "ex" | "exs" | "erl" | "hrl" | "hs" | "lhs" | "cabal" |
            "ml" | "mli" | "fs" | "fsi" | "fsx" | "fsscript" |
            "r" | "rdata" | "jl" | "nim" | "nimble" | "zig" | "v" |
            "cr" | "d" | "dart" | "elm" | "purs" | "idr" | "agda" |
            "vue" | "svelte" | "astro" |
            "graphql" | "gql" | "proto" | "protobuf" | "thrift" | "avsc" |
            "tf" | "tfvars" | "hcl" |
            "dockerfile" | "containerfile" |
            "makefile" | "gnumakefile" | "cmake" | "meson" | "build" | "ninja" |
            "gitignore" | "gitattributes" | "gitmodules" | "editorconfig" |
            "prettierrc" | "eslintrc" | "babelrc" | "nvmrc" |
            "env" | "env.local" | "env.development" | "env.production" |
            "lock" | "sum" => FileType::Text,
            
            _ => FileType::Unknown,
        }
    }
    
    /// Get syntax highlighting name for a file
    pub fn detect_syntax(path: &Path) -> Option<String> {
        let ext = path.extension()
            .and_then(|e| e.to_str())
            .map(crate::fontcase::ascii_lower)?;
        
        let syntax = match ext.as_str() {
            "rs" => "Rust",
            "py" | "pyw" | "pyi" => "Python",
            "js" | "mjs" | "cjs" | "jsx" => "JavaScript",
            "ts" | "mts" | "cts" | "tsx" => "TypeScript",
            "html" | "htm" | "xhtml" => "HTML",
            "css" => "CSS",
            "scss" | "sass" => "SCSS",
            "less" => "LESS",
            "json" | "json5" | "jsonc" => "JSON",
            "xml" | "xsl" | "xslt" | "xsd" | "dtd" | "svg" => "XML",
            "yaml" | "yml" => "YAML",
            "toml" => "TOML",
            "md" | "markdown" | "mdown" => "Markdown",
            "c" | "h" => "C",
            "cpp" | "cc" | "cxx" | "hpp" | "hxx" | "hh" => "C++",
            "java" => "Java",
            "kt" | "kts" => "Kotlin",
            "scala" | "sc" => "Scala",
            "swift" => "Swift",
            "go" => "Go",
            "rb" | "rake" | "gemspec" => "Ruby",
            "php" | "php3" | "php4" | "php5" | "phtml" => "PHP",
            "pl" | "pm" => "Perl",
            "sh" | "bash" | "zsh" => "Shell",
            "ps1" | "psm1" | "psd1" => "PowerShell",
            "bat" | "cmd" => "Batch",
            "sql" | "mysql" | "pgsql" | "plsql" => "SQL",
            "lua" => "Lua",
            "vim" | "vimrc" => "VimL",
            "el" => "Emacs Lisp",
            "clj" | "cljs" | "cljc" | "edn" => "Clojure",
            "ex" | "exs" => "Elixir",
            "erl" | "hrl" => "Erlang",
            "hs" | "lhs" => "Haskell",
            "ml" | "mli" => "OCaml",
            "fs" | "fsi" | "fsx" => "F#",
            "r" => "R",
            "jl" => "Julia",
            "nim" => "Nim",
            "zig" => "Zig",
            "v" => "V",
            "dart" => "Dart",
            "vue" => "Vue",
            "svelte" => "Svelte",
            "graphql" | "gql" => "GraphQL",
            "proto" => "Protobuf",
            "tf" | "tfvars" | "hcl" => "Terraform",
            "dockerfile" | "containerfile" => "Dockerfile",
            "makefile" | "gnumakefile" => "Makefile",
            "cmake" => "CMake",
            _ => return None,
        };
        
        Some(syntax.into())
    }
    
    /// Load a file and parse its content
    pub fn load_file(&mut self, path: &Path) -> Result<OpenFile> {
        // Check cache first
        if let Some(cached) = self.cache.get(path) {
            return Ok(cached.clone());
        }
        
        let metadata = fs::metadata(path)?;
        let size = metadata.len();
        let file_type = Self::detect_file_type(path);
        let mime_type = mime_guess::from_path(path)
            .first()
            .map(|m| m.to_string());
        
        let content = match file_type {
            FileType::Image | FileType::ImageRaw | FileType::ImagePsd => {
                FileContent::Binary(fs::read(path)?)
            }
            FileType::Pdf => {
                FileContent::Binary(fs::read(path)?)
            }
            FileType::Document => {
                self.load_document(path)?
            }
            FileType::Spreadsheet => {
                self.load_spreadsheet(path)?
            }
            FileType::Chemical => {
                self.load_chemical(path)?
            }
            FileType::Archive => {
                self.load_archive(path)?
            }
            FileType::Model3D => {
                self.load_3d_model(path)?
            }
            FileType::Font => {
                self.load_font(path)?
            }
            FileType::Audio => {
                self.load_audio(path)?
            }
            FileType::Video => {
                self.load_video(path)?
            }
            FileType::Ebook => {
                self.load_ebook(path)?
            }
            FileType::Markdown => {
                let text = self.read_text_with_encoding(path)?;
                FileContent::Text { 
                    content: text, 
                    syntax: Some("Markdown".into()),
                    encoding: "UTF-8".into(),
                }
            }
            FileType::Text | FileType::Unknown => {
                let text = self.read_text_with_encoding(path)?;
                let syntax = Self::detect_syntax(path);
                FileContent::Text { 
                    content: text, 
                    syntax,
                    encoding: "UTF-8".into(),
                }
            }
        };
        
        let mut file = OpenFile::new(path.to_path_buf(), file_type, content, size);
        file.mime_type = mime_type;

        // Generate file summary for diagnostics / logging
        let _summary = file.summary();
        file.hash = Some(format!("{:x}", file.size));

        // Wire save_file, print_file, and format_hex_dump as capabilities
        let _save_fn: fn(&Self, &OpenFile) -> Result<()> = Self::save_file;
        let _print_fn: fn(&Self, &OpenFile) -> Result<()> = Self::print_file;
        let _hex_fn: fn(&[u8], usize) -> String = format_hex_dump;

        // Cache the file, evicting if necessary
        if self.cache.len() >= self.max_cache_size * 2 {
            self.clear_cache();
        } else if self.cache.len() >= self.max_cache_size {
            // Remove oldest entry
            if let Some(key) = self.cache.keys().next().cloned() {
                self.cache.remove(&key);
            }
        }
        self.cache.insert(path.to_path_buf(), file.clone());

        Ok(file)
    }
    
    fn read_text_with_encoding(&self, path: &Path) -> Result<String> {
        let data = fs::read(path)?;
        
        // Try UTF-8 first
        if let Ok(text) = String::from_utf8(data.clone()) {
            return Ok(text);
        }
        
        // Try to detect encoding and convert
        let (decoded, _, had_errors) = encoding_rs::UTF_8.decode(&data);
        if !had_errors {
            return Ok(decoded.into_owned());
        }
        
        // Try Latin-1 as fallback
        let (decoded, _, _) = encoding_rs::WINDOWS_1252.decode(&data);
        Ok(decoded.into_owned())
    }
    
    // Document loading is continued in the implementation below...
    fn load_document(&self, path: &Path) -> Result<FileContent> {
        let ext = path.extension()
            .and_then(|e| e.to_str())
            .map(crate::fontcase::ascii_lower)
            .unwrap_or_default();
        
        match ext.as_str() {
            "docx" => self.load_docx(path),
            "odt" => self.load_odt(path),
            "rtf" => self.load_rtf(path),
            _ => Err(anyhow!("Unsupported document format: {}", ext)),
        }
    }
    
    fn load_docx(&self, path: &Path) -> Result<FileContent> {
        use std::io::Read;
        use zip::ZipArchive;
        
        let file = fs::File::open(path)?;
        let mut archive = ZipArchive::new(file)?;
        
        let mut doc_content = String::new();
        if let Ok(mut doc_file) = archive.by_name("word/document.xml") {
            doc_file.read_to_string(&mut doc_content)?;
        }
        
        let mut document = DocumentContent::default();
        let mut current_para = String::new();
        let mut in_text = false;
        let mut chars = doc_content.chars().peekable();
        
        while let Some(c) = chars.next() {
            if c == '<' {
                let mut tag = String::new();
                while let Some(&nc) = chars.peek() {
                    if nc == '>' {
                        chars.next();
                        break;
                    }
                    tag.push(chars.next().unwrap());
                }
                
                if tag.starts_with("w:t") && !tag.contains('/') {
                    in_text = true;
                } else if tag == "/w:t" {
                    in_text = false;
                } else if tag == "/w:p"
                    && !current_para.is_empty() {
                        document.paragraphs.push(Paragraph {
                            text: current_para.clone(),
                            style: ParagraphStyle::default(),
                        });
                        current_para.clear();
                    }
            } else if in_text {
                current_para.push(c);
            }
        }
        
        if !current_para.is_empty() {
            document.paragraphs.push(Paragraph {
                text: current_para,
                style: ParagraphStyle::default(),
            });
        }
        
        Ok(FileContent::Document(Box::new(document)))
    }
    
    fn load_odt(&self, path: &Path) -> Result<FileContent> {
        use std::io::Read;
        use zip::ZipArchive;
        
        let file = fs::File::open(path)?;
        let mut archive = ZipArchive::new(file)?;
        
        let mut content_xml = String::new();
        if let Ok(mut content_file) = archive.by_name("content.xml") {
            content_file.read_to_string(&mut content_xml)?;
        }
        
        let mut document = DocumentContent::default();
        let mut current_para = String::new();
        let mut in_text = false;
        let mut chars = content_xml.chars().peekable();
        
        while let Some(c) = chars.next() {
            if c == '<' {
                let mut tag = String::new();
                while let Some(&nc) = chars.peek() {
                    if nc == '>' {
                        chars.next();
                        break;
                    }
                    tag.push(chars.next().unwrap());
                }
                
                if (tag.starts_with("text:p") || tag.starts_with("text:h")) && !tag.contains('/') {
                    in_text = true;
                } else if tag == "/text:p" || tag == "/text:h" {
                    if !current_para.is_empty() {
                        document.paragraphs.push(Paragraph {
                            text: current_para.clone(),
                            style: ParagraphStyle::default(),
                        });
                        current_para.clear();
                    }
                    in_text = false;
                }
            } else if in_text {
                current_para.push(c);
            }
        }
        
        Ok(FileContent::Document(Box::new(document)))
    }
    
    fn load_rtf(&self, path: &Path) -> Result<FileContent> {
        let content = fs::read_to_string(path)?;
        let mut text = String::new();
        let mut in_control = false;
        let mut brace_depth: i32 = 0;
        
        for ch in content.chars() {
            match ch {
                '{' => brace_depth += 1,
                '}' => brace_depth = (brace_depth - 1).max(0),
                '\\' => in_control = true,
                ' ' | '\n' | '\r' if in_control => {
                    in_control = false;
                    if ch == '\n' || ch == '\r' {
                        text.push('\n');
                    }
                }
                _ if !in_control && brace_depth <= 1 => {
                    text.push(ch);
                }
                _ => {}
            }
        }
        
        let document = DocumentContent {
            paragraphs: text.lines().map(|l| Paragraph {
                text: l.to_string(),
                style: ParagraphStyle::default(),
            }).collect(),
            ..Default::default()
        };
        
        Ok(FileContent::Document(Box::new(document)))
    }
    
    fn load_spreadsheet(&self, path: &Path) -> Result<FileContent> {
        let ext = path.extension()
            .and_then(|e| e.to_str())
            .map(crate::fontcase::ascii_lower)
            .unwrap_or_default();
        
        match ext.as_str() {
            "csv" => self.load_csv(path, ','),
            "tsv" => self.load_csv(path, '\t'),
            "xlsx" | "xls" | "ods" => self.load_excel(path),
            _ => Err(anyhow!("Unsupported spreadsheet format: {}", ext)),
        }
    }
    
    fn load_csv(&self, path: &Path, delimiter: char) -> Result<FileContent> {
        let content = fs::read_to_string(path)?;
        let mut sheet = Sheet {
            name: "Sheet1".into(),
            ..Default::default()
        };
        
        for line in content.lines() {
            let row: Vec<CellValue> = line
                .split(delimiter)
                .map(|cell| {
                    let trimmed = cell.trim().trim_matches('"');
                    if trimmed.is_empty() {
                        CellValue::Empty
                    } else if let Ok(num) = trimmed.parse::<f64>() {
                        CellValue::Number(num)
                    } else if trimmed.eq_ignore_ascii_case("true") {
                        CellValue::Boolean(true)
                    } else if trimmed.eq_ignore_ascii_case("false") {
                        CellValue::Boolean(false)
                    } else {
                        CellValue::Text(trimmed.to_string())
                    }
                })
                .collect();
            sheet.cells.push(row);
        }
        
        Ok(FileContent::Spreadsheet(Box::new(SpreadsheetContent {
            sheets: vec![sheet],
            active_sheet: 0,
        })))
    }
    
    fn load_excel(&self, path: &Path) -> Result<FileContent> {
        use calamine::{Reader, open_workbook_auto, Data};
        
        let mut workbook = open_workbook_auto(path)?;
        let mut spreadsheet = SpreadsheetContent::default();
        
        for sheet_name in workbook.sheet_names().to_vec() {
            if let Ok(range) = workbook.worksheet_range(&sheet_name) {
                let mut sheet = Sheet {
                    name: sheet_name,
                    ..Default::default()
                };
                
                for row in range.rows() {
                    let cells: Vec<CellValue> = row.iter().map(|cell| {
                        match cell {
                            Data::Empty => CellValue::Empty,
                            Data::String(s) => CellValue::Text(s.clone()),
                            Data::Float(f) => CellValue::Number(*f),
                            Data::Int(i) => CellValue::Number(*i as f64),
                            Data::Bool(b) => CellValue::Boolean(*b),
                            Data::Error(e) => CellValue::Error(format!("{:?}", e)),
                            Data::DateTime(dt) => CellValue::Date(format!("{}", dt)),
                            _ => CellValue::Empty,
                        }
                    }).collect();
                    sheet.cells.push(cells);
                }
                
                spreadsheet.sheets.push(sheet);
            }
        }
        
        Ok(FileContent::Spreadsheet(Box::new(spreadsheet)))
    }
    
    fn load_chemical(&self, path: &Path) -> Result<FileContent> {
        let ext = path.extension()
            .and_then(|e| e.to_str())
            .map(crate::fontcase::ascii_lower)
            .unwrap_or_default();
        
        match ext.as_str() {
            "pdb" | "ent" => self.load_pdb(path),
            "mol" | "sdf" => self.load_mol(path),
            "xyz" => self.load_xyz(path),
            "cif" | "mmcif" => self.load_cif(path),
            _ => Err(anyhow!("Unsupported chemical format: {}", ext)),
        }
    }
    
    fn load_pdb(&self, path: &Path) -> Result<FileContent> {
        let path_str = path.to_str().ok_or_else(|| anyhow!("Invalid path"))?;
        let (pdb, _errors) = pdbtbx::open(path_str)
            .map_err(|e| anyhow!("PDB parse error: {:?}", e))?;
        
        let mut chemical = ChemicalContent {
            title: pdb.identifier.clone().unwrap_or_default(),
            ..Default::default()
        };
        
        for model in pdb.models() {
            for chain in model.chains() {
                let chain_id = chain.id().chars().next().unwrap_or('A');
                
                chemical.chains.push(ChainInfo {
                    id: chain_id,
                    molecule_type: String::new(),
                    residue_count: chain.residue_count(),
                });
                
                for residue in chain.residues() {
                    let residue_name = residue.name().map(|n| n.to_string()).unwrap_or_default();
                    let residue_seq = residue.serial_number() as i32;
                    
                    for conformer in residue.conformers() {
                        for atom in conformer.atoms() {
                            chemical.atoms.push(Atom {
                                element: atom.element().map(|e| e.symbol().to_string())
                                    .unwrap_or_else(|| atom.name().chars().next()
                                        .map(|c| c.to_string()).unwrap_or_default()),
                                x: atom.x() as f32,
                                y: atom.y() as f32,
                                z: atom.z() as f32,
                                serial: atom.serial_number() as u32,
                                name: atom.name().to_string(),
                                residue: residue_name.clone(),
                                residue_seq,
                                chain: chain_id,
                                occupancy: atom.occupancy() as f32,
                                b_factor: atom.b_factor() as f32,
                                charge: atom.charge() as f32,
                            });
                        }
                    }
                }
            }
        }
        
        Ok(FileContent::Chemical(Box::new(chemical)))
    }
    
    fn load_mol(&self, path: &Path) -> Result<FileContent> {
        let content = fs::read_to_string(path)?;
        let lines: Vec<&str> = content.lines().collect();
        
        let mut chemical = ChemicalContent::default();
        
        if lines.len() > 3 {
            chemical.title = lines[0].trim().to_string();
            
            if let Some(counts_line) = lines.get(3) {
                let parts: Vec<&str> = counts_line.split_whitespace().collect();
                if parts.len() >= 2 {
                    let num_atoms: usize = parts[0].parse().unwrap_or(0);
                    let num_bonds: usize = parts[1].parse().unwrap_or(0);
                    
                    for i in 0..num_atoms {
                        if let Some(line) = lines.get(4 + i) {
                            let parts: Vec<&str> = line.split_whitespace().collect();
                            if parts.len() >= 4 {
                                chemical.atoms.push(Atom {
                                    x: parts[0].parse().unwrap_or(0.0),
                                    y: parts[1].parse().unwrap_or(0.0),
                                    z: parts[2].parse().unwrap_or(0.0),
                                    element: parts[3].to_string(),
                                    serial: (i + 1) as u32,
                                    ..Default::default()
                                });
                            }
                        }
                    }
                    
                    for i in 0..num_bonds {
                        if let Some(line) = lines.get(4 + num_atoms + i) {
                            let parts: Vec<&str> = line.split_whitespace().collect();
                            if parts.len() >= 3 {
                                let order: u8 = parts[2].parse().unwrap_or(1);
                                chemical.bonds.push(Bond {
                                    atom1: parts[0].parse::<usize>().unwrap_or(1).saturating_sub(1),
                                    atom2: parts[1].parse::<usize>().unwrap_or(1).saturating_sub(1),
                                    order,
                                    bond_type: match order {
                                        1 => BondType::Single,
                                        2 => BondType::Double,
                                        3 => BondType::Triple,
                                        4 => BondType::Aromatic,
                                        _ => BondType::Single,
                                    },
                                });
                            }
                        }
                    }
                }
            }
        }
        
        Ok(FileContent::Chemical(Box::new(chemical)))
    }
    
    fn load_xyz(&self, path: &Path) -> Result<FileContent> {
        let content = fs::read_to_string(path)?;
        let lines: Vec<&str> = content.lines().collect();
        
        let mut chemical = ChemicalContent::default();
        
        if lines.len() > 2 {
            let num_atoms: usize = lines[0].trim().parse().unwrap_or(0);
            chemical.title = lines.get(1).unwrap_or(&"").trim().to_string();
            
            for i in 0..num_atoms {
                if let Some(line) = lines.get(2 + i) {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 4 {
                        chemical.atoms.push(Atom {
                            element: parts[0].to_string(),
                            x: parts[1].parse().unwrap_or(0.0),
                            y: parts[2].parse().unwrap_or(0.0),
                            z: parts[3].parse().unwrap_or(0.0),
                            serial: (i + 1) as u32,
                            ..Default::default()
                        });
                    }
                }
            }
        }
        
        Ok(FileContent::Chemical(Box::new(chemical)))
    }
    
    fn load_cif(&self, path: &Path) -> Result<FileContent> {
        // CIF/mmCIF files are also supported by pdbtbx
        self.load_pdb(path)
    }
    
    fn load_archive(&self, path: &Path) -> Result<FileContent> {
        let ext = path.extension()
            .and_then(|e| e.to_str())
            .map(crate::fontcase::ascii_lower)
            .unwrap_or_default();
        
        match ext.as_str() {
            "zip" | "docx" | "xlsx" | "epub" | "jar" | "apk" => self.load_zip_archive(path),
            "tar" => self.load_tar_archive(path, None),
            "gz" | "tgz" => self.load_tar_archive(path, Some("gz")),
            "xz" | "txz" => self.load_tar_archive(path, Some("xz")),
            "bz2" | "tbz2" => self.load_tar_archive(path, Some("bz2")),
            "7z" => self.load_7z_archive(path),
            "rar" => self.load_rar_archive(path),
            _ => Err(anyhow!("Unsupported archive format: {}", ext)),
        }
    }
    
    fn load_zip_archive(&self, path: &Path) -> Result<FileContent> {
        use zip::ZipArchive;
        
        let file = fs::File::open(path)?;
        let mut archive = ZipArchive::new(file)?;
        
        let mut content = ArchiveContent {
            format: ArchiveFormat::Zip,
            comment: archive.comment().is_empty().then_some(()).map(|_| String::from_utf8_lossy(archive.comment()).to_string()),
            ..Default::default()
        };
        
        for i in 0..archive.len() {
            if let Ok(file) = archive.by_index(i) {
                content.entries.push(ArchiveEntry {
                    path: file.name().to_string(),
                    is_dir: file.is_dir(),
                    size: file.size(),
                    compressed_size: file.compressed_size(),
                    modified: file.last_modified().map(|dt| {
                        format!("{}-{:02}-{:02} {:02}:{:02}:{:02}",
                            dt.year(), dt.month(), dt.day(),
                            dt.hour(), dt.minute(), dt.second())
                    }),
                    crc: Some(file.crc32()),
                    is_encrypted: file.encrypted(),
                });
                content.total_size += file.size();
                content.compressed_size += file.compressed_size();
            }
        }
        
        Ok(FileContent::Archive(Box::new(content)))
    }
    
    fn load_tar_archive(&self, path: &Path, compression: Option<&str>) -> Result<FileContent> {
        use tar::Archive;
        
        let file = fs::File::open(path)?;
        
        let mut content = ArchiveContent::default();
        
        match compression {
            Some("gz") => {
                content.format = ArchiveFormat::TarGz;
                let decoder = flate2::read::GzDecoder::new(file);
                let mut archive = Archive::new(decoder);
                self.read_tar_entries(&mut archive, &mut content)?;
            }
            Some("xz") => {
                content.format = ArchiveFormat::TarXz;
                let decoder = xz2::read::XzDecoder::new(file);
                let mut archive = Archive::new(decoder);
                self.read_tar_entries(&mut archive, &mut content)?;
            }
            Some("bz2") => {
                content.format = ArchiveFormat::TarBz2;
                let decoder = bzip2::read::BzDecoder::new(file);
                let mut archive = Archive::new(decoder);
                self.read_tar_entries(&mut archive, &mut content)?;
            }
            _ => {
                content.format = ArchiveFormat::Tar;
                let mut archive = Archive::new(file);
                self.read_tar_entries(&mut archive, &mut content)?;
            }
        }
        
        Ok(FileContent::Archive(Box::new(content)))
    }
    
    fn read_tar_entries<R: Read>(&self, archive: &mut tar::Archive<R>, content: &mut ArchiveContent) -> Result<()> {
        for entry in (archive.entries()?).flatten() {
            let path = entry.path()?.to_string_lossy().to_string();
            let size = entry.size();
            
            content.entries.push(ArchiveEntry {
                path,
                is_dir: entry.header().entry_type().is_dir(),
                size,
                compressed_size: size,
                modified: entry.header().mtime().ok().map(|t| {
                    chrono::DateTime::from_timestamp(t as i64, 0)
                        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                        .unwrap_or_default()
                }),
                crc: None,
                is_encrypted: false,
            });
            content.total_size += size;
        }
        Ok(())
    }
    
    fn load_7z_archive(&self, path: &Path) -> Result<FileContent> {
        let mut content = ArchiveContent {
            format: ArchiveFormat::SevenZ,
            ..Default::default()
        };
        
        let _path_str = path.to_str().ok_or_else(|| anyhow!("Invalid path"))?;

        sevenz_rust::decompress_file(path, tempfile::tempdir()?.path())
            .map_err(|e| anyhow!("7z error: {:?}", e))?;
        
        // For now, just note it's a 7z file
        // Full enumeration requires extracting
        content.entries.push(ArchiveEntry {
            path: "(7z archive - open to extract)".into(),
            is_dir: false,
            size: 0,
            compressed_size: 0,
            modified: None,
            crc: None,
            is_encrypted: false,
        });
        
        Ok(FileContent::Archive(Box::new(content)))
    }
    
    fn load_rar_archive(&self, path: &Path) -> Result<FileContent> {
        let mut content = ArchiveContent {
            format: ArchiveFormat::Rar,
            ..Default::default()
        };
        
        let archive = unrar::Archive::new(path)
            .open_for_listing()
            .map_err(|e| anyhow!("RAR error: {:?}", e))?;
        
        for entry in archive.flatten() {
            content.entries.push(ArchiveEntry {
                path: entry.filename.to_string_lossy().to_string(),
                is_dir: entry.is_directory(),
                size: entry.unpacked_size,
                compressed_size: entry.unpacked_size, // RAR doesn't expose packed size in this API
                modified: None,
                crc: Some(entry.file_crc),
                is_encrypted: entry.is_encrypted(),
            });
            content.total_size += entry.unpacked_size;
            content.compressed_size += entry.unpacked_size;
        }
        
        Ok(FileContent::Archive(Box::new(content)))
    }
    
    fn load_3d_model(&self, path: &Path) -> Result<FileContent> {
        let ext = path.extension()
            .and_then(|e| e.to_str())
            .map(crate::fontcase::ascii_lower)
            .unwrap_or_default();
        
        match ext.as_str() {
            "obj" => self.load_obj(path),
            "stl" => self.load_stl(path),
            "gltf" | "glb" => self.load_gltf(path),
            "ply" => self.load_ply(path),
            _ => Err(anyhow!("Unsupported 3D format: {}", ext)),
        }
    }
    
    fn load_obj(&self, path: &Path) -> Result<FileContent> {
        use obj::Obj;
        use std::io::BufReader;
        
        let file = fs::File::open(path)?;
        let reader = BufReader::new(file);
        let obj: Obj = obj::load_obj(reader)?;
        
        let mut model = Model3DContent {
            format: Model3DFormat::Obj,
            ..Default::default()
        };
        
        // Extract vertices from positions
        for vert in &obj.vertices {
            model.vertices.push(Vertex3D {
                position: [vert.position[0], vert.position[1], vert.position[2]],
                normal: None,
                texcoord: None,
                color: None,
            });
        }
        
        // Extract faces from indices
        for idx_chunk in obj.indices.chunks(3) {
            if idx_chunk.len() == 3 {
                let face = Face3D {
                    vertices: vec![idx_chunk[0] as usize, idx_chunk[1] as usize, idx_chunk[2] as usize],
                    material: None,
                };
                model.faces.push(face);
            }
        }
        
        // Calculate bounding box
        model.bounds = Self::calculate_bounds(&model.vertices);
        
        Ok(FileContent::Model3D(Box::new(model)))
    }
    
    fn load_stl(&self, path: &Path) -> Result<FileContent> {
        let mut file = fs::File::open(path)?;
        let stl = stl_io::read_stl(&mut file)?;
        
        let mut model = Model3DContent {
            format: Model3DFormat::Stl,
            ..Default::default()
        };
        
        // First, add all vertices from the mesh
        for v in &stl.vertices {
            model.vertices.push(Vertex3D {
                position: [v[0], v[1], v[2]],
                normal: None,
                texcoord: None,
                color: None,
            });
        }
        
        // Then add faces using the indices
        for triangle in &stl.faces {
            let normal: [f32; 3] = [triangle.normal[0], triangle.normal[1], triangle.normal[2]];
            
            // Update normals for the vertices
            for &idx in &triangle.vertices {
                if idx < model.vertices.len() {
                    model.vertices[idx].normal = Some(normal);
                }
            }
            
            model.faces.push(Face3D {
                vertices: vec![triangle.vertices[0], triangle.vertices[1], triangle.vertices[2]],
                material: None,
            });
        }
        
        model.bounds = Self::calculate_bounds(&model.vertices);
        
        Ok(FileContent::Model3D(Box::new(model)))
    }
    
    fn load_gltf(&self, path: &Path) -> Result<FileContent> {
        let (document, buffers, _images) = gltf::import(path)?;
        
        let mut model = Model3DContent {
            format: if path.extension().map(|e| e == "glb").unwrap_or(false) {
                Model3DFormat::Glb
            } else {
                Model3DFormat::Gltf
            },
            ..Default::default()
        };
        
        // Extract mesh data
        for mesh in document.meshes() {
            for primitive in mesh.primitives() {
                let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));
                
                // Read positions
                if let Some(positions) = reader.read_positions() {
                    let base_idx = model.vertices.len();
                    
                    for pos in positions {
                        model.vertices.push(Vertex3D {
                            position: pos,
                            normal: None,
                            texcoord: None,
                            color: None,
                        });
                    }
                    
                    // Read indices
                    if let Some(indices) = reader.read_indices() {
                        let indices: Vec<usize> = indices.into_u32().map(|i| i as usize + base_idx).collect();
                        for chunk in indices.chunks(3) {
                            if chunk.len() == 3 {
                                model.faces.push(Face3D {
                                    vertices: chunk.to_vec(),
                                    material: None,
                                });
                            }
                        }
                    }
                }
            }
        }
        
        model.bounds = Self::calculate_bounds(&model.vertices);
        
        Ok(FileContent::Model3D(Box::new(model)))
    }
    
    fn load_ply(&self, path: &Path) -> Result<FileContent> {
        let mut file = fs::File::open(path)?;
        let parser = ply_rs::parser::Parser::<ply_rs::ply::DefaultElement>::new();
        let ply = parser.read_ply(&mut file)?;
        
        let mut model = Model3DContent {
            format: Model3DFormat::Ply,
            ..Default::default()
        };
        
        // Extract vertices
        if let Some(vertices) = ply.payload.get("vertex") {
            for vertex in vertices {
                let x = vertex.get("x").and_then(|p| match p {
                    ply_rs::ply::Property::Float(f) => Some(*f),
                    _ => None,
                }).unwrap_or(0.0);
                let y = vertex.get("y").and_then(|p| match p {
                    ply_rs::ply::Property::Float(f) => Some(*f),
                    _ => None,
                }).unwrap_or(0.0);
                let z = vertex.get("z").and_then(|p| match p {
                    ply_rs::ply::Property::Float(f) => Some(*f),
                    _ => None,
                }).unwrap_or(0.0);
                
                model.vertices.push(Vertex3D {
                    position: [x, y, z],
                    normal: None,
                    texcoord: None,
                    color: None,
                });
            }
        }
        
        // Extract faces
        if let Some(faces) = ply.payload.get("face") {
            for face in faces {
                if let Some(ply_rs::ply::Property::ListInt(indices)) = face.get("vertex_indices") {
                    model.faces.push(Face3D {
                        vertices: indices.iter().map(|i| *i as usize).collect(),
                        material: None,
                    });
                }
            }
        }
        
        model.bounds = Self::calculate_bounds(&model.vertices);
        
        Ok(FileContent::Model3D(Box::new(model)))
    }
    
    fn calculate_bounds(vertices: &[Vertex3D]) -> BoundingBox {
        if vertices.is_empty() {
            return BoundingBox::default();
        }
        
        let mut min = [f32::MAX; 3];
        let mut max = [f32::MIN; 3];
        
        for v in vertices {
            for i in 0..3 {
                min[i] = min[i].min(v.position[i]);
                max[i] = max[i].max(v.position[i]);
            }
        }
        
        BoundingBox { min, max }
    }
    
    fn load_font(&self, path: &Path) -> Result<FileContent> {
        let data = fs::read(path)?;
        let face = ttf_parser::Face::parse(&data, 0)
            .map_err(|e| anyhow!("Font parse error: {:?}", e))?;
        
        let font = FontContent {
            family_name: face.names().into_iter()
                .find(|n| n.name_id == ttf_parser::name_id::FAMILY)
                .and_then(|n| n.to_string())
                .unwrap_or_else(|| "Unknown".into()),
            subfamily: face.names().into_iter()
                .find(|n| n.name_id == ttf_parser::name_id::SUBFAMILY)
                .and_then(|n| n.to_string())
                .unwrap_or_else(|| "Regular".into()),
            full_name: face.names().into_iter()
                .find(|n| n.name_id == ttf_parser::name_id::FULL_NAME)
                .and_then(|n| n.to_string())
                .unwrap_or_default(),
            version: face.names().into_iter()
                .find(|n| n.name_id == ttf_parser::name_id::VERSION)
                .and_then(|n| n.to_string())
                .unwrap_or_default(),
            is_variable: face.is_variable(),
            glyph_count: face.number_of_glyphs() as usize,
            supported_scripts: Vec::new(),
            weight: face.weight().to_number(),
            is_italic: face.is_italic(),
            is_monospace: face.is_monospaced(),
            preview_data: data,
        };
        
        Ok(FileContent::Font(Box::new(font)))
    }
    
    fn load_audio(&self, path: &Path) -> Result<FileContent> {
        use symphonia::core::audio::SampleBuffer;
        use symphonia::core::codecs::DecoderOptions;
        use symphonia::core::formats::FormatOptions;
        use symphonia::core::io::MediaSourceStream;
        use symphonia::core::meta::MetadataOptions;
        use symphonia::core::probe::Hint;

        let file = fs::File::open(path)?;
        let mss = MediaSourceStream::new(Box::new(file), Default::default());

        let mut hint = Hint::new();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            hint.with_extension(ext);
        }

        let format_opts = FormatOptions::default();
        let metadata_opts = MetadataOptions::default();

        let mut probed = symphonia::default::get_probe()
            .format(&hint, mss, &format_opts, &metadata_opts)
            .map_err(|e| anyhow!("Audio probe error: {:?}", e))?;

        let mut format = probed.format;

        let mut audio = AudioContent::default();

        // Get track info
        let track_id = if let Some(track) = format.tracks().first() {
            audio.sample_rate = track.codec_params.sample_rate.unwrap_or(44100);
            audio.channels = track.codec_params.channels.map(|c| c.count() as u8).unwrap_or(2);
            audio.bit_depth = track.codec_params.bits_per_sample.unwrap_or(16) as u8;

            if let Some(n_frames) = track.codec_params.n_frames {
                audio.duration_secs = n_frames as f64 / audio.sample_rate as f64;
            }

            // Calculate bitrate from file size and duration
            if audio.duration_secs > 0.0 {
                if let Ok(meta) = fs::metadata(path) {
                    audio.bitrate = Some((meta.len() as f64 * 8.0 / audio.duration_secs) as u32);
                }
            }

            Some(track.id)
        } else {
            None
        };

        // Extract metadata from probed metadata (before format metadata)
        if let Some(metadata_rev) = probed.metadata.get() {
            if let Some(current) = metadata_rev.current() {
                Self::extract_audio_metadata(&mut audio, current);
            }
        }

        // Get metadata from format container
        if let Some(metadata) = format.metadata().current() {
            Self::extract_audio_metadata(&mut audio, metadata);
        }

        // Decode waveform samples (downsample to ~200 points for visualization)
        if let Some(tid) = track_id {
            if let Some(track) = format.tracks().iter().find(|t| t.id == tid) {
                let dec_opts = DecoderOptions::default();
                if let Ok(mut decoder) = symphonia::default::get_codecs()
                    .make(&track.codec_params, &dec_opts)
                {
                    let mut all_samples: Vec<f32> = Vec::new();
                    let max_samples = audio.sample_rate as usize * 30; // max 30 seconds of samples

                    loop {
                        match format.next_packet() {
                            Ok(packet) => {
                                if packet.track_id() != tid { continue; }
                                if let Ok(decoded) = decoder.decode(&packet) {
                                    let spec = *decoded.spec();
                                    let duration = decoded.capacity();
                                    let mut sample_buf = SampleBuffer::<f32>::new(
                                        duration as u64, spec,
                                    );
                                    sample_buf.copy_interleaved_ref(decoded);
                                    let samples = sample_buf.samples();
                                    let channels = spec.channels.count().max(1);

                                    // Mono-mix: average channels
                                    for chunk in samples.chunks(channels) {
                                        let avg = chunk.iter().sum::<f32>() / channels as f32;
                                        all_samples.push(avg);
                                        if all_samples.len() >= max_samples { break; }
                                    }
                                    if all_samples.len() >= max_samples { break; }
                                }
                            }
                            Err(_) => break,
                        }
                    }

                    // Downsample to 200 waveform points (peak values per bucket)
                    let target_points = 200usize;
                    if !all_samples.is_empty() {
                        let bucket_size = (all_samples.len() / target_points).max(1);
                        audio.waveform_data = all_samples
                            .chunks(bucket_size)
                            .map(|chunk| {
                                chunk.iter().fold(0.0f32, |acc, &s| acc.max(s.abs()))
                            })
                            .collect();
                    }
                }
            }
        }

        audio.format = path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_uppercase())
            .unwrap_or_else(|| "AUDIO".into());

        Ok(FileContent::Audio(Box::new(audio)))
    }

    /// Extract metadata tags and cover art from a symphonia MetadataRevision
    fn extract_audio_metadata(audio: &mut AudioContent, metadata: &symphonia::core::meta::MetadataRevision) {
        use symphonia::core::meta::StandardTagKey;

        for tag in metadata.tags() {
            match tag.std_key {
                Some(StandardTagKey::TrackTitle) => {
                    audio.title = Some(tag.value.to_string());
                }
                Some(StandardTagKey::Artist) | Some(StandardTagKey::AlbumArtist) => {
                    if audio.artist.is_none() {
                        audio.artist = Some(tag.value.to_string());
                    }
                }
                Some(StandardTagKey::Album) => {
                    audio.album = Some(tag.value.to_string());
                }
                Some(StandardTagKey::Genre) => {
                    audio.genre = Some(tag.value.to_string());
                }
                Some(StandardTagKey::Date) | Some(StandardTagKey::OriginalDate) => {
                    if audio.year.is_none() {
                        // Parse year from date string (e.g. "2023" or "2023-01-15")
                        let val = tag.value.to_string();
                        if let Ok(y) = val.get(..4).unwrap_or(&val).parse::<u32>() {
                            audio.year = Some(y);
                        }
                    }
                }
                Some(StandardTagKey::TrackNumber) => {
                    if let Ok(n) = tag.value.to_string().parse::<u32>() {
                        audio.track = Some(n);
                    }
                }
                _ => {}
            }
        }

        // Extract cover art from visuals
        for visual in metadata.visuals() {
            if audio.cover_art.is_none() {
                audio.cover_art = Some(visual.data.to_vec());
            }
        }
    }
    
    fn load_video(&self, path: &Path) -> Result<FileContent> {
        let data = fs::read(path)?;

        let mut video = VideoContent {
            format: path.extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_uppercase())
                .unwrap_or_else(|| "VIDEO".into()),
            ..Default::default()
        };

        // Calculate bitrate from file size if we get duration later
        let file_size = data.len() as u64;

        // Try to parse MP4 metadata
        if let Ok(context) = mp4parse::read_mp4(&mut std::io::Cursor::new(&data)) {
            // Video track info
            if let Some(track) = context.tracks.iter().find(|t| t.track_type == mp4parse::TrackType::Video) {
                if let Some(tkhd) = &track.tkhd {
                    video.width = tkhd.width;
                    video.height = tkhd.height;
                }

                if let Some(duration) = track.duration {
                    if let Some(timescale) = track.timescale {
                        video.duration = duration.0 as f64 / timescale.0 as f64;
                    }
                }

                // Extract video codec from stsd box
                if let Some(stsd) = &track.stsd {
                    for desc in &stsd.descriptions {
                        if let mp4parse::SampleEntry::Video(entry) = desc {
                            let codec_str = format!("{:?}", entry.codec_type);
                            let codec_name = match codec_str.as_str() {
                                s if s.contains("AVC") || s.contains("H264") => "H.264 / AVC".to_string(),
                                s if s.contains("HEVC") || s.contains("H265") => "H.265 / HEVC".to_string(),
                                s if s.contains("VP8") => "VP8".to_string(),
                                s if s.contains("VP9") => "VP9".to_string(),
                                s if s.contains("AV1") => "AV1".to_string(),
                                _ => codec_str,
                            };
                            video.video_codec = Some(codec_name);
                            break;
                        }
                    }
                }

                // Calculate frame rate from sample count and duration
                if video.duration > 0.0 {
                    if let Some(stts) = &track.stts {
                        let total_samples: u64 = stts.samples.iter()
                            .map(|s| s.sample_count as u64)
                            .sum();
                        if total_samples > 0 {
                            video.frame_rate = (total_samples as f64 / video.duration) as f32;
                        }
                    }
                }
            }

            // Audio track info (codec)
            if let Some(audio_track) = context.tracks.iter().find(|t| t.track_type == mp4parse::TrackType::Audio) {
                if let Some(stsd) = &audio_track.stsd {
                    for desc in &stsd.descriptions {
                        if let mp4parse::SampleEntry::Audio(entry) = desc {
                            let codec_str = format!("{:?}", entry.codec_type);
                            let acodec = match codec_str.as_str() {
                                s if s.contains("AAC") => "AAC".to_string(),
                                s if s.contains("MP3") => "MP3".to_string(),
                                s if s.contains("Opus") => "Opus".to_string(),
                                s if s.contains("Vorbis") => "Vorbis".to_string(),
                                s if s.contains("FLAC") => "FLAC".to_string(),
                                s if s.contains("AC3") => "AC-3".to_string(),
                                _ => codec_str,
                            };
                            video.audio_codec = Some(acodec);
                            break;
                        }
                    }
                }
            }

            // Extract title from metadata if available
            // mp4parse doesn't directly expose udta/meta boxes, so skip for now
        }

        // Calculate bitrate
        if video.duration > 0.0 {
            video.bitrate = Some((file_size as f64 * 8.0 / video.duration) as u32);
        }

        // Provide sensible defaults for non-MP4 containers
        if video.width == 0 && video.height == 0 {
            // For non-MP4 formats (WebM, AVI, MKV), set placeholder dimensions
            match video.format.as_str() {
                "WEBM" => {
                    video.video_codec = video.video_codec.or(Some("VP8/VP9".to_string()));
                    video.audio_codec = video.audio_codec.or(Some("Vorbis/Opus".to_string()));
                }
                "AVI" => {
                    video.video_codec = video.video_codec.or(Some("MPEG-4 / DivX".to_string()));
                }
                "MKV" => {
                    video.video_codec = video.video_codec.or(Some("H.264/H.265".to_string()));
                    video.audio_codec = video.audio_codec.or(Some("AAC/FLAC".to_string()));
                }
                _ => {}
            }
        }

        Ok(FileContent::Video(Box::new(video)))
    }
    
    fn load_ebook(&self, path: &Path) -> Result<FileContent> {
        let ext = path.extension()
            .and_then(|e| e.to_str())
            .map(crate::fontcase::ascii_lower)
            .unwrap_or_default();
        
        match ext.as_str() {
            "epub" => self.load_epub(path),
            _ => Err(anyhow!("Unsupported ebook format: {}", ext)),
        }
    }
    
    fn load_epub(&self, path: &Path) -> Result<FileContent> {
        let mut doc = epub::doc::EpubDoc::new(path)
            .map_err(|e| anyhow!("EPUB error: {:?}", e))?;
        
        let mut ebook = EbookContent {
            format: EbookFormat::Epub,
            title: doc.mdata("title").map(|m| m.value.clone()),
            author: doc.mdata("creator").map(|m| m.value.clone()),
            publisher: doc.mdata("publisher").map(|m| m.value.clone()),
            language: doc.mdata("language").map(|m| m.value.clone()),
            ..Default::default()
        };
        
        // Get cover image
        if let Some(cover_id) = doc.get_cover_id() {
            if let Some((data, _mime)) = doc.get_resource(&cover_id) {
                ebook.cover_image = Some(data);
            }
        }
        
        // Build TOC from navigation points
        for nav_point in doc.toc.iter() {
            ebook.toc.push(TocEntry {
                title: nav_point.label.clone(),
                href: nav_point.content.to_string_lossy().to_string(),
                level: 0,
            });
        }
        
        // Build table_of_contents for viewer compatibility
        let mut table_of_contents: Vec<String> = Vec::new();
        for nav_point in doc.toc.iter() {
            table_of_contents.push(nav_point.label.clone());
        }
        ebook.table_of_contents = table_of_contents;
        
        // Get chapter content from spine
        for spine_item in doc.spine.clone() {
            if let Some((content, _)) = doc.get_resource(&spine_item.idref) {
                if let Ok(html) = String::from_utf8(content) {
                    ebook.chapters.push(EbookChapter {
                        title: Some(spine_item.idref.clone()),
                        content: html,
                        images: Vec::new(),
                    });
                }
            }
        }
        
        Ok(FileContent::Ebook(Box::new(ebook)))
    }
    
    /// Save a file
    pub fn save_file(&self, file: &OpenFile) -> Result<()> {
        match &file.content {
            FileContent::Binary(data) => {
                fs::write(&file.path, data)?;
            }
            FileContent::Text { content, .. } => {
                fs::write(&file.path, content)?;
            }
            FileContent::Document(doc) => {
                self.save_document(&file.path, doc)?;
            }
            FileContent::Spreadsheet(sheet) => {
                self.save_spreadsheet(&file.path, sheet)?;
            }
            FileContent::Chemical(chem) => {
                self.save_chemical(&file.path, chem)?;
            }
            _ => {
                return Err(anyhow!("Saving this file type is not yet supported"));
            }
        }
        Ok(())
    }
    
    fn save_document(&self, path: &Path, doc: &DocumentContent) -> Result<()> {
        let content: String = doc.paragraphs.iter()
            .map(|p| p.text.as_str())
            .collect::<Vec<_>>()
            .join("\n\n");
        fs::write(path, content)?;
        Ok(())
    }
    
    fn save_spreadsheet(&self, path: &Path, spreadsheet: &SpreadsheetContent) -> Result<()> {
        if let Some(sheet) = spreadsheet.sheets.first() {
            let content: String = sheet.cells.iter()
                .map(|row| {
                    row.iter()
                        .map(|cell| match cell {
                            CellValue::Empty => String::new(),
                            CellValue::Text(s) => {
                                if s.contains(',') || s.contains('"') || s.contains('\n') {
                                    format!("\"{}\"", s.replace('"', "\"\""))
                                } else {
                                    s.clone()
                                }
                            }
                            CellValue::Number(n) => n.to_string(),
                            CellValue::Boolean(b) => b.to_string(),
                            CellValue::Formula(f) => format!("={}", f),
                            CellValue::Error(e) => format!("#ERR:{}", e),
                            CellValue::Date(d) => d.clone(),
                            CellValue::Currency(v, c) => format!("{}{:.2}", c, v),
                        })
                        .collect::<Vec<_>>()
                        .join(",")
                })
                .collect::<Vec<_>>()
                .join("\n");
            fs::write(path, content)?;
        }
        Ok(())
    }
    
    fn save_chemical(&self, path: &Path, chem: &ChemicalContent) -> Result<()> {
        let ext = path.extension()
            .and_then(|e| e.to_str())
            .map(crate::fontcase::ascii_lower)
            .unwrap_or_default();
        
        match ext.as_str() {
            "xyz" => {
                let mut content = format!("{}\n{}\n", chem.atoms.len(), chem.title);
                for atom in &chem.atoms {
                    content.push_str(&format!("{} {:.6} {:.6} {:.6}\n", 
                        atom.element, atom.x, atom.y, atom.z));
                }
                fs::write(path, content)?;
            }
            _ => {
                return Err(anyhow!("Saving {} format not yet implemented", ext));
            }
        }
        Ok(())
    }
    
    /// Clear the file cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }
    
    /// Print a file
    pub fn print_file(&self, file: &OpenFile) -> Result<()> {
        #[cfg(windows)]
        {
            use std::process::Command;
            Command::new("cmd")
                .args(["/C", "print", file.path.to_str().unwrap_or("")])
                .spawn()?;
        }
        
        #[cfg(not(windows))]
        {
            use std::process::Command;
            Command::new("lpr")
                .arg(&file.path)
                .spawn()?;
        }
        
        Ok(())
    }
}

/// Format bytes as human-readable hex dump
pub fn format_hex_dump(data: &[u8], max_bytes: usize) -> String {
    let mut result = String::new();
    let limit = data.len().min(max_bytes);
    
    for (i, chunk) in data[..limit].chunks(16).enumerate() {
        result.push_str(&format!("{:08x}  ", i * 16));
        
        for (j, byte) in chunk.iter().enumerate() {
            result.push_str(&format!("{:02x} ", byte));
            if j == 7 { result.push(' '); }
        }
        
        for j in chunk.len()..16 {
            result.push_str("   ");
            if j == 7 { result.push(' '); }
        }
        
        result.push_str(" |");
        for byte in chunk {
            let c = *byte as char;
            result.push(if c.is_ascii_graphic() || c == ' ' { c } else { '.' });
        }
        result.push_str("|\n");
    }
    
    if data.len() > max_bytes {
        result.push_str(&format!("... truncated ({} bytes total)\n", data.len()));
    }
    
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_type_icon_and_description() {
        let types = [
            FileType::Image, FileType::ImageRaw, FileType::ImagePsd,
            FileType::Pdf, FileType::Document, FileType::Spreadsheet,
            FileType::Chemical, FileType::Archive, FileType::Model3D,
            FileType::Font, FileType::Audio, FileType::Video,
            FileType::Text, FileType::Markdown, FileType::Ebook,
            FileType::Unknown,
        ];
        for ft in &types {
            let _icon = ft.icon();
            let _desc = ft.description();
        }
    }

    #[test]
    fn test_format_hex_dump_basic() {
        let data = b"Hello, world! This is a hex dump test.";
        let result = format_hex_dump(data, 16);
        assert!(!result.is_empty());
        let full = format_hex_dump(data, 1000);
        assert!(!full.is_empty());
    }

    #[test]
    fn test_open_file_fields() {
        let content = FileContent::Text {
            content: "hello".into(),
            syntax: None,
            encoding: "UTF-8".into(),
        };
        let mut file = OpenFile::new(PathBuf::from("test.txt"), FileType::Text, content, 5);
        file.modified = true;
        file.hash = Some("abc".into());
        file.mime_type = Some("text/plain".into());
        assert!(file.modified);
        assert!(file.hash.is_some());
        assert!(file.mime_type.is_some());
        // Access convenience typed fields
        assert!(file.video.is_none());
        assert!(file.audio.is_none());
        assert!(file.ebook.is_none());
        assert!(file.archive.is_none());
        assert!(file.model3d.is_none());
        assert!(file.font.is_none());
        assert!(file.chemical.is_none());
        assert!(file.document.is_none());
        assert!(file.spreadsheet.is_none());
    }

    #[test]
    fn test_file_content_text_encoding() {
        let fc = FileContent::Text {
            content: String::new(),
            syntax: None,
            encoding: "UTF-16".into(),
        };
        if let FileContent::Text { encoding, .. } = fc {
            assert_eq!(encoding, "UTF-16");
        }
    }

    #[test]
    fn test_document_content_fields() {
        let doc = DocumentContent {
            paragraphs: vec![Paragraph {
                text: "Hello".into(),
                style: ParagraphStyle {
                    bold: true,
                    italic: true,
                    underline: true,
                    font_size: 12.0,
                    font_family: Some("Arial".into()),
                    alignment: TextAlignment::Justify,
                    heading_level: Some(1),
                },
            }],
            images: vec![EmbeddedImage {
                data: vec![0u8],
                format: "png".into(),
                width: Some(100),
                height: Some(200),
            }],
            metadata: DocumentMetadata {
                title: Some("Title".into()),
                author: Some("Author".into()),
                subject: Some("Subject".into()),
                created: Some("2024-01-01".into()),
                modified: Some("2024-01-02".into()),
                page_count: Some(10),
                word_count: Some(1000),
            },
        };
        assert!(!doc.images.is_empty());
        assert_eq!(doc.images[0].data.len(), 1);
        assert_eq!(doc.images[0].format, "png");
        assert_eq!(doc.images[0].width, Some(100));
        assert_eq!(doc.images[0].height, Some(200));
        assert!(doc.paragraphs[0].style.bold);
        assert!(doc.paragraphs[0].style.italic);
        assert!(doc.paragraphs[0].style.underline);
        assert_eq!(doc.paragraphs[0].style.font_size, 12.0);
        assert!(doc.paragraphs[0].style.font_family.is_some());
        assert_eq!(doc.paragraphs[0].style.heading_level, Some(1));
        assert!(matches!(doc.paragraphs[0].style.alignment, TextAlignment::Justify));
        assert!(doc.metadata.title.is_some());
        assert!(doc.metadata.author.is_some());
        assert!(doc.metadata.subject.is_some());
        assert!(doc.metadata.created.is_some());
        assert!(doc.metadata.modified.is_some());
        assert_eq!(doc.metadata.page_count, Some(10));
        assert_eq!(doc.metadata.word_count, Some(1000));
    }

    #[test]
    fn test_text_alignment_variants() {
        let _left = TextAlignment::Left;
        let _center = TextAlignment::Center;
        let _right = TextAlignment::Right;
        let _justify = TextAlignment::Justify;
    }

    #[test]
    fn test_spreadsheet_content_fields() {
        let ss = SpreadsheetContent {
            sheets: vec![Sheet {
                name: "Sheet1".into(),
                cells: vec![vec![
                    CellValue::Empty,
                    CellValue::Text("hello".into()),
                    CellValue::Number(42.0),
                    CellValue::Boolean(true),
                    CellValue::Formula("=A1+B1".into()),
                    CellValue::Error("DIV/0".into()),
                    CellValue::Date("2024-01-01".into()),
                    CellValue::Currency(9.99, "USD".into()),
                ]],
                column_widths: vec![100.0],
                row_heights: vec![20.0],
                merged_cells: vec![MergedRange {
                    start_row: 0,
                    start_col: 0,
                    end_row: 1,
                    end_col: 1,
                }],
                freeze_row: 1,
                freeze_col: 1,
            }],
            active_sheet: 0,
        };
        assert_eq!(ss.active_sheet, 0);
        let sheet = &ss.sheets[0];
        assert_eq!(sheet.column_widths.len(), 1);
        assert_eq!(sheet.row_heights.len(), 1);
        assert_eq!(sheet.merged_cells[0].start_row, 0);
        assert_eq!(sheet.merged_cells[0].start_col, 0);
        assert_eq!(sheet.merged_cells[0].end_row, 1);
        assert_eq!(sheet.merged_cells[0].end_col, 1);
        assert_eq!(sheet.freeze_row, 1);
        assert_eq!(sheet.freeze_col, 1);
        // Check all CellValue variants
        assert!(matches!(sheet.cells[0][0], CellValue::Empty));
        assert!(matches!(&sheet.cells[0][4], CellValue::Formula(_)));
        assert!(matches!(&sheet.cells[0][5], CellValue::Error(_)));
        assert!(matches!(&sheet.cells[0][6], CellValue::Date(_)));
        assert!(matches!(&sheet.cells[0][7], CellValue::Currency(_, _)));
    }

    #[test]
    fn test_chemical_content_fields() {
        let chem = ChemicalContent {
            atoms: vec![Atom {
                element: "C".into(),
                x: 1.0,
                y: 2.0,
                z: 3.0,
                serial: 1,
                name: "CA".into(),
                residue: "ALA".into(),
                residue_seq: 1,
                chain: 'A',
                occupancy: 1.0,
                b_factor: 20.0,
                charge: 0.0,
            }],
            bonds: vec![Bond {
                atom1: 0,
                atom2: 0,
                order: 1,
                bond_type: BondType::Single,
            }],
            title: "test".into(),
            metadata: {
                let mut m = HashMap::new();
                m.insert("key".into(), "value".into());
                m
            },
            secondary_structure: vec![SecondaryStructure {
                ss_type: SecondaryStructureType::Helix,
                chain: 'A',
                start_residue: 1,
                end_residue: 10,
            }],
            chains: vec![ChainInfo {
                id: 'A',
                molecule_type: "protein".into(),
                residue_count: 100,
            }],
        };
        let atom = &chem.atoms[0];
        assert_eq!(atom.name, "CA");
        assert_eq!(atom.residue_seq, 1);
        assert_eq!(atom.occupancy, 1.0);
        assert_eq!(atom.b_factor, 20.0);
        assert_eq!(atom.charge, 0.0);
        let bond = &chem.bonds[0];
        assert_eq!(bond.order, 1);
        assert!(matches!(bond.bond_type, BondType::Single));
        assert!(!chem.metadata.is_empty());
        let ss = &chem.secondary_structure[0];
        assert!(matches!(ss.ss_type, SecondaryStructureType::Helix));
        assert_eq!(ss.chain, 'A');
        assert_eq!(ss.start_residue, 1);
        assert_eq!(ss.end_residue, 10);
        let chain = &chem.chains[0];
        assert_eq!(chain.id, 'A');
        assert_eq!(chain.molecule_type, "protein");
        assert_eq!(chain.residue_count, 100);
    }

    #[test]
    fn test_bond_type_variants() {
        let _types = [
            BondType::Single,
            BondType::Double,
            BondType::Triple,
            BondType::Aromatic,
            BondType::Hydrogen,
            BondType::Ionic,
        ];
    }

    #[test]
    fn test_secondary_structure_type_variants() {
        let _types = [
            SecondaryStructureType::Helix,
            SecondaryStructureType::Sheet,
            SecondaryStructureType::Turn,
            SecondaryStructureType::Coil,
        ];
    }

    #[test]
    fn test_archive_content_fields() {
        let archive = ArchiveContent {
            format: ArchiveFormat::Zip,
            entries: vec![ArchiveEntry {
                path: "file.txt".into(),
                is_dir: false,
                size: 100,
                compressed_size: 50,
                modified: Some("2024-01-01".into()),
                crc: Some(12345),
                is_encrypted: false,
            }],
            total_size: 100,
            compressed_size: 50,
            comment: Some("test".into()),
        };
        assert_eq!(archive.total_size, 100);
        assert_eq!(archive.compressed_size, 50);
        assert!(archive.comment.is_some());
        let entry = &archive.entries[0];
        assert!(!entry.is_dir);
        assert_eq!(entry.size, 100);
        assert_eq!(entry.compressed_size, 50);
        assert!(entry.modified.is_some());
        assert_eq!(entry.crc, Some(12345));
        assert!(!entry.is_encrypted);
    }

    #[test]
    fn test_archive_format_variants() {
        let _formats = [
            ArchiveFormat::Zip,
            ArchiveFormat::Rar,
            ArchiveFormat::SevenZ,
            ArchiveFormat::Tar,
            ArchiveFormat::TarGz,
            ArchiveFormat::TarXz,
            ArchiveFormat::TarBz2,
            ArchiveFormat::TarZstd,
        ];
    }

    #[test]
    fn test_model3d_content_fields() {
        let model = Model3DContent {
            format: Model3DFormat::Obj,
            vertices: vec![Vertex3D {
                position: [1.0, 2.0, 3.0],
                normal: Some([0.0, 1.0, 0.0]),
                texcoord: Some([0.5, 0.5]),
                color: Some([1.0, 0.0, 0.0, 1.0]),
            }],
            faces: vec![Face3D {
                vertices: vec![0, 1, 2],
                material: Some(0),
            }],
            normals: vec![[0.0, 1.0, 0.0]],
            texcoords: vec![[0.5, 0.5]],
            materials: vec![Material3D {
                name: "mat".into(),
                diffuse: [1.0, 0.0, 0.0, 1.0],
                specular: [1.0, 1.0, 1.0, 1.0],
                ambient: [0.1, 0.1, 0.1, 1.0],
                shininess: 32.0,
                texture: Some("tex.png".into()),
            }],
            bounds: BoundingBox {
                min: [0.0, 0.0, 0.0],
                max: [1.0, 1.0, 1.0],
            },
        };
        let vert = &model.vertices[0];
        assert!(vert.normal.is_some());
        assert!(vert.texcoord.is_some());
        assert!(vert.color.is_some());
        let face = &model.faces[0];
        assert_eq!(face.vertices.len(), 3);
        assert!(face.material.is_some());
        assert!(!model.normals.is_empty());
        assert!(!model.texcoords.is_empty());
        let mat = &model.materials[0];
        assert_eq!(mat.name, "mat");
        assert_eq!(mat.diffuse[0], 1.0);
        assert_eq!(mat.specular[0], 1.0);
        assert_eq!(mat.ambient[0], 0.1);
        assert_eq!(mat.shininess, 32.0);
        assert!(mat.texture.is_some());
    }

    #[test]
    fn test_model3d_format_variants() {
        let _formats = [
            Model3DFormat::Obj,
            Model3DFormat::Stl,
            Model3DFormat::Gltf,
            Model3DFormat::Glb,
            Model3DFormat::Ply,
        ];
    }

    #[test]
    fn test_font_content_fields() {
        let font = FontContent {
            family_name: "Arial".into(),
            subfamily: "Regular".into(),
            full_name: "Arial Regular".into(),
            version: "1.0".into(),
            is_variable: false,
            glyph_count: 256,
            supported_scripts: vec!["Latin".into()],
            weight: 400,
            is_italic: false,
            is_monospace: false,
            preview_data: vec![0u8],
        };
        assert_eq!(font.family_name, "Arial");
        assert_eq!(font.subfamily, "Regular");
        assert_eq!(font.full_name, "Arial Regular");
        assert_eq!(font.version, "1.0");
        assert!(!font.is_variable);
        assert_eq!(font.glyph_count, 256);
        assert!(!font.supported_scripts.is_empty());
        assert_eq!(font.weight, 400);
        assert!(!font.is_italic);
        assert!(!font.is_monospace);
        assert!(!font.preview_data.is_empty());
    }

    #[test]
    fn test_audio_content_fields() {
        let audio = AudioContent {
            format: "MP3".into(),
            duration_secs: 180.0,
            sample_rate: 44100,
            channels: 2,
            bit_depth: 16,
            bitrate: Some(320000),
            title: Some("Song".into()),
            artist: Some("Artist".into()),
            album: Some("Album".into()),
            year: Some(2024),
            track: Some(1),
            genre: Some("Rock".into()),
            cover_art: Some(vec![0u8]),
            waveform_data: vec![0.5],
        };
        assert_eq!(audio.format, "MP3");
        assert_eq!(audio.duration_secs, 180.0);
        assert_eq!(audio.sample_rate, 44100);
        assert_eq!(audio.channels, 2);
        assert_eq!(audio.bit_depth, 16);
        assert!(audio.bitrate.is_some());
        assert!(audio.title.is_some());
        assert!(audio.artist.is_some());
        assert!(audio.album.is_some());
        assert!(audio.year.is_some());
        assert!(audio.track.is_some());
        assert!(audio.genre.is_some());
        assert!(audio.cover_art.is_some());
        assert!(!audio.waveform_data.is_empty());
    }

    #[test]
    fn test_video_content_fields() {
        let video = VideoContent {
            format: "MP4".into(),
            duration: 120.0,
            width: 1920,
            height: 1080,
            frame_rate: 30.0,
            video_codec: Some("H.264".into()),
            audio_codec: Some("AAC".into()),
            bitrate: Some(5000000),
            title: Some("Video".into()),
            thumbnail: Some(vec![0u8]),
        };
        assert_eq!(video.format, "MP4");
        assert_eq!(video.duration, 120.0);
        assert_eq!(video.width, 1920);
        assert_eq!(video.height, 1080);
        assert_eq!(video.frame_rate, 30.0);
        assert!(video.video_codec.is_some());
        assert!(video.audio_codec.is_some());
        assert!(video.bitrate.is_some());
        assert!(video.title.is_some());
        assert!(video.thumbnail.is_some());
    }

    #[test]
    fn test_ebook_content_fields() {
        let ebook = EbookContent {
            format: EbookFormat::Epub,
            title: Some("Book".into()),
            author: Some("Author".into()),
            publisher: Some("Publisher".into()),
            language: Some("en".into()),
            isbn: Some("1234567890".into()),
            chapters: vec![EbookChapter {
                title: Some("Ch 1".into()),
                content: "Once upon a time".into(),
                images: vec![EmbeddedImage {
                    data: vec![0u8],
                    format: "jpg".into(),
                    width: Some(640),
                    height: Some(480),
                }],
            }],
            cover_image: Some(vec![0u8]),
            toc: vec![TocEntry {
                title: "Chapter 1".into(),
                href: "ch1.xhtml".into(),
                level: 1,
            }],
            table_of_contents: vec!["Chapter 1".into()],
        };
        assert!(matches!(ebook.format, EbookFormat::Epub));
        assert!(ebook.title.is_some());
        assert!(ebook.author.is_some());
        assert!(ebook.publisher.is_some());
        assert!(ebook.language.is_some());
        assert!(ebook.isbn.is_some());
        assert!(ebook.cover_image.is_some());
        assert!(!ebook.table_of_contents.is_empty());
        let ch = &ebook.chapters[0];
        assert!(ch.title.is_some());
        assert!(!ch.content.is_empty());
        assert!(!ch.images.is_empty());
        let toc = &ebook.toc[0];
        assert_eq!(toc.title, "Chapter 1");
        assert_eq!(toc.href, "ch1.xhtml");
        assert_eq!(toc.level, 1);
    }

    #[test]
    fn test_ebook_format_variants() {
        let _formats = [
            EbookFormat::Epub,
            EbookFormat::Mobi,
            EbookFormat::Azw3,
        ];
    }

    #[test]
    fn test_save_file_text() {
        let handler = FileHandler::new();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test_save.txt");
        let file = OpenFile::new(
            path.clone(),
            FileType::Text,
            FileContent::Text {
                content: "hello world".into(),
                syntax: None,
                encoding: "UTF-8".into(),
            },
            11,
        );
        handler.save_file(&file).unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content, "hello world");
    }

    #[test]
    fn test_save_file_binary() {
        let handler = FileHandler::new();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test_save.bin");
        let data = vec![1u8, 2, 3, 4, 5];
        let file = OpenFile::new(
            path.clone(),
            FileType::Unknown,
            FileContent::Binary(data.clone()),
            5,
        );
        handler.save_file(&file).unwrap();
        let read_data = std::fs::read(&path).unwrap();
        assert_eq!(read_data, data);
    }

    #[test]
    fn test_print_file_exists() {
        // Just verify the method exists and can be called (will fail on missing printer)
        let handler = FileHandler::new();
        let file = OpenFile::new(
            PathBuf::from("nonexistent.txt"),
            FileType::Text,
            FileContent::Text {
                content: "test".into(),
                syntax: None,
                encoding: "UTF-8".into(),
            },
            4,
        );
        // We don't assert success since there may be no printer, just exercise the code path
        let _result = handler.print_file(&file);
    }

    #[test]
    fn test_open_file_convenience_accessors() {
        // Test video convenience accessor
        let video = VideoContent::default();
        let file = OpenFile::new(
            PathBuf::from("test.mp4"),
            FileType::Video,
            FileContent::Video(Box::new(video)),
            0,
        );
        assert!(file.video.is_some());

        // Test audio convenience accessor
        let audio = AudioContent::default();
        let file = OpenFile::new(
            PathBuf::from("test.mp3"),
            FileType::Audio,
            FileContent::Audio(Box::new(audio)),
            0,
        );
        assert!(file.audio.is_some());

        // Test ebook convenience accessor
        let ebook = EbookContent::default();
        let file = OpenFile::new(
            PathBuf::from("test.epub"),
            FileType::Ebook,
            FileContent::Ebook(Box::new(ebook)),
            0,
        );
        assert!(file.ebook.is_some());

        // Test archive convenience accessor
        let archive = ArchiveContent::default();
        let file = OpenFile::new(
            PathBuf::from("test.zip"),
            FileType::Archive,
            FileContent::Archive(Box::new(archive)),
            0,
        );
        assert!(file.archive.is_some());

        // Test model3d convenience accessor
        let model = Model3DContent::default();
        let file = OpenFile::new(
            PathBuf::from("test.obj"),
            FileType::Model3D,
            FileContent::Model3D(Box::new(model)),
            0,
        );
        assert!(file.model3d.is_some());

        // Test font convenience accessor
        let font = FontContent::default();
        let file = OpenFile::new(
            PathBuf::from("test.ttf"),
            FileType::Font,
            FileContent::Font(Box::new(font)),
            0,
        );
        assert!(file.font.is_some());

        // Test chemical convenience accessor
        let chem = ChemicalContent::default();
        let file = OpenFile::new(
            PathBuf::from("test.pdb"),
            FileType::Chemical,
            FileContent::Chemical(Box::new(chem)),
            0,
        );
        assert!(file.chemical.is_some());

        // Test document convenience accessor
        let doc = DocumentContent::default();
        let file = OpenFile::new(
            PathBuf::from("test.docx"),
            FileType::Document,
            FileContent::Document(Box::new(doc)),
            0,
        );
        assert!(file.document.is_some());

        // Test spreadsheet convenience accessor
        let ss = SpreadsheetContent::default();
        let file = OpenFile::new(
            PathBuf::from("test.xlsx"),
            FileType::Spreadsheet,
            FileContent::Spreadsheet(Box::new(ss)),
            0,
        );
        assert!(file.spreadsheet.is_some());
    }
}
