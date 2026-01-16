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
use std::io::{Read, Seek};
use std::path::{Path, PathBuf};

// ═══════════════════════════════════════════════════════════════════════════════
// FILE TYPE ENUMERATION
// ═══════════════════════════════════════════════════════════════════════════════

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
            Self::Image | Self::ImageRaw | Self::ImagePsd => "🖼️",
            Self::Pdf => "📕",
            Self::Document => "📄",
            Self::Spreadsheet => "📊",
            Self::Chemical => "🧬",
            Self::Archive => "📦",
            Self::Model3D => "🎲",
            Self::Font => "🔤",
            Self::Audio => "🎵",
            Self::Video => "🎬",
            Self::Text => "📝",
            Self::Markdown => "📑",
            Self::Ebook => "📚",
            Self::Unknown => "📁",
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

// ═══════════════════════════════════════════════════════════════════════════════
// OPEN FILE STRUCTURE
// ═══════════════════════════════════════════════════════════════════════════════

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
        
        // Extract typed content for convenience
        let video = match &content {
            FileContent::Video(v) => Some(v.clone()),
            _ => None,
        };
        let audio = match &content {
            FileContent::Audio(a) => Some(a.clone()),
            _ => None,
        };
        let ebook = match &content {
            FileContent::Ebook(e) => Some(e.clone()),
            _ => None,
        };
        let archive = match &content {
            FileContent::Archive(a) => Some(a.clone()),
            _ => None,
        };
        let model3d = match &content {
            FileContent::Model3D(m) => Some(m.clone()),
            _ => None,
        };
        let font = match &content {
            FileContent::Font(f) => Some(f.clone()),
            _ => None,
        };
        let chemical = match &content {
            FileContent::Chemical(c) => Some(c.clone()),
            _ => None,
        };
        let document = match &content {
            FileContent::Document(d) => Some(d.clone()),
            _ => None,
        };
        let spreadsheet = match &content {
            FileContent::Spreadsheet(s) => Some(s.clone()),
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
}

#[derive(Debug, Clone)]
pub enum FileContent {
    Binary(Vec<u8>),
    Text { content: String, syntax: Option<String>, encoding: String },
    Document(DocumentContent),
    Spreadsheet(SpreadsheetContent),
    Chemical(ChemicalContent),
    Archive(ArchiveContent),
    Model3D(Model3DContent),
    Font(FontContent),
    Audio(AudioContent),
    Video(VideoContent),
    Ebook(EbookContent),
}

// ═══════════════════════════════════════════════════════════════════════════════
// DOCUMENT CONTENT
// ═══════════════════════════════════════════════════════════════════════════════

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

// ═══════════════════════════════════════════════════════════════════════════════
// SPREADSHEET CONTENT
// ═══════════════════════════════════════════════════════════════════════════════

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
pub enum CellValue {
    Empty,
    Text(String),
    Number(f64),
    Boolean(bool),
    Formula(String),
    Error(String),
    Date(String),
    Currency(f64, String),
}

impl Default for CellValue {
    fn default() -> Self {
        CellValue::Empty
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// CHEMICAL CONTENT
// ═══════════════════════════════════════════════════════════════════════════════

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

// ═══════════════════════════════════════════════════════════════════════════════
// ARCHIVE CONTENT
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Default)]
pub struct ArchiveContent {
    pub format: ArchiveFormat,
    pub entries: Vec<ArchiveEntry>,
    pub total_size: u64,
    pub compressed_size: u64,
    pub comment: Option<String>,
}

#[derive(Debug, Clone, Default)]
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

// ═══════════════════════════════════════════════════════════════════════════════
// 3D MODEL CONTENT
// ═══════════════════════════════════════════════════════════════════════════════

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

// ═══════════════════════════════════════════════════════════════════════════════
// FONT CONTENT
// ═══════════════════════════════════════════════════════════════════════════════

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

// ═══════════════════════════════════════════════════════════════════════════════
// AUDIO CONTENT
// ═══════════════════════════════════════════════════════════════════════════════

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

// ═══════════════════════════════════════════════════════════════════════════════
// VIDEO CONTENT
// ═══════════════════════════════════════════════════════════════════════════════

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

// ═══════════════════════════════════════════════════════════════════════════════
// EBOOK CONTENT
// ═══════════════════════════════════════════════════════════════════════════════

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
    pub table_of_contents: Vec<String>,
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

// ═══════════════════════════════════════════════════════════════════════════════
// FILE HANDLER
// ═══════════════════════════════════════════════════════════════════════════════

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
            .map(|e| e.to_lowercase())
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
            "m4v" | "mpeg" | "mpg" | "3gp" | "ogv" | "ts" | "mts" => FileType::Video,
            
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
            .map(|e| e.to_lowercase())?;
        
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
        
        let file = OpenFile {
            path: path.to_path_buf(),
            file_type,
            content,
            size,
            modified: false,
            mime_type,
            hash: None,
        };
        
        // Cache the file
        if self.cache.len() >= self.max_cache_size {
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
            .map(|e| e.to_lowercase())
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
                } else if tag == "/w:p" {
                    if !current_para.is_empty() {
                        document.paragraphs.push(Paragraph {
                            text: current_para.clone(),
                            style: ParagraphStyle::default(),
                        });
                        current_para.clear();
                    }
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
        
        Ok(FileContent::Document(document))
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
        
        Ok(FileContent::Document(document))
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
        
        Ok(FileContent::Document(document))
    }
    
    fn load_spreadsheet(&self, path: &Path) -> Result<FileContent> {
        let ext = path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
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
        
        Ok(FileContent::Spreadsheet(SpreadsheetContent {
            sheets: vec![sheet],
            active_sheet: 0,
        }))
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
        
        Ok(FileContent::Spreadsheet(spreadsheet))
    }
    
    fn load_chemical(&self, path: &Path) -> Result<FileContent> {
        let ext = path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
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
        
        Ok(FileContent::Chemical(chemical))
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
        
        Ok(FileContent::Chemical(chemical))
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
        
        Ok(FileContent::Chemical(chemical))
    }
    
    fn load_cif(&self, path: &Path) -> Result<FileContent> {
        // CIF/mmCIF files are also supported by pdbtbx
        self.load_pdb(path)
    }
    
    fn load_archive(&self, path: &Path) -> Result<FileContent> {
        let ext = path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
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
            comment: archive.comment().is_empty().then_some(()).map_or(
                None, 
                |_| Some(String::from_utf8_lossy(archive.comment()).to_string())
            ),
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
        
        Ok(FileContent::Archive(content))
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
        
        Ok(FileContent::Archive(content))
    }
    
    fn read_tar_entries<R: Read>(&self, archive: &mut tar::Archive<R>, content: &mut ArchiveContent) -> Result<()> {
        for entry in archive.entries()? {
            if let Ok(entry) = entry {
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
        }
        Ok(())
    }
    
    fn load_7z_archive(&self, path: &Path) -> Result<FileContent> {
        let mut content = ArchiveContent {
            format: ArchiveFormat::SevenZ,
            ..Default::default()
        };
        
        let path_str = path.to_str().ok_or_else(|| anyhow!("Invalid path"))?;
        
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
        
        Ok(FileContent::Archive(content))
    }
    
    fn load_rar_archive(&self, path: &Path) -> Result<FileContent> {
        let mut content = ArchiveContent {
            format: ArchiveFormat::Rar,
            ..Default::default()
        };
        
        let archive = unrar::Archive::new(path)
            .list()
            .map_err(|e| anyhow!("RAR error: {:?}", e))?;
        
        for entry in archive {
            if let Ok(entry) = entry {
                content.entries.push(ArchiveEntry {
                    path: entry.filename.to_string_lossy().to_string(),
                    is_dir: entry.is_directory(),
                    size: entry.unpacked_size as u64,
                    compressed_size: entry.packed_size as u64,
                    modified: None,
                    crc: Some(entry.file_crc),
                    is_encrypted: entry.is_encrypted(),
                });
                content.total_size += entry.unpacked_size as u64;
                content.compressed_size += entry.packed_size as u64;
            }
        }
        
        Ok(FileContent::Archive(content))
    }
    
    fn load_3d_model(&self, path: &Path) -> Result<FileContent> {
        let ext = path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
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
        let obj = obj_rs::Obj::load(path)?;
        
        let mut model = Model3DContent {
            format: Model3DFormat::Obj,
            ..Default::default()
        };
        
        // Extract vertices
        for pos in &obj.data.position {
            model.vertices.push(Vertex3D {
                position: [pos[0], pos[1], pos[2]],
                normal: None,
                texcoord: None,
                color: None,
            });
        }
        
        // Extract faces
        for object in &obj.data.objects {
            for group in &object.groups {
                for poly in &group.polys {
                    let face = Face3D {
                        vertices: poly.0.iter().map(|idx| idx.0).collect(),
                        material: None,
                    };
                    model.faces.push(face);
                }
            }
        }
        
        // Calculate bounding box
        model.bounds = Self::calculate_bounds(&model.vertices);
        
        Ok(FileContent::Model3D(model))
    }
    
    fn load_stl(&self, path: &Path) -> Result<FileContent> {
        let mut file = fs::File::open(path)?;
        let stl = stl_io::read_stl(&mut file)?;
        
        let mut model = Model3DContent {
            format: Model3DFormat::Stl,
            ..Default::default()
        };
        
        // STL stores triangles directly
        for triangle in stl.faces {
            let base_idx = model.vertices.len();
            
            for vertex in &triangle.vertices {
                model.vertices.push(Vertex3D {
                    position: [vertex[0], vertex[1], vertex[2]],
                    normal: Some(triangle.normal),
                    texcoord: None,
                    color: None,
                });
            }
            
            model.faces.push(Face3D {
                vertices: vec![base_idx, base_idx + 1, base_idx + 2],
                material: None,
            });
        }
        
        model.bounds = Self::calculate_bounds(&model.vertices);
        
        Ok(FileContent::Model3D(model))
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
        
        Ok(FileContent::Model3D(model))
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
        
        Ok(FileContent::Model3D(model))
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
            is_monospace: face.is_monospace(),
            preview_data: data,
        };
        
        Ok(FileContent::Font(font))
    }
    
    fn load_audio(&self, path: &Path) -> Result<FileContent> {
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
        
        let probed = symphonia::default::get_probe()
            .format(&hint, mss, &format_opts, &metadata_opts)
            .map_err(|e| anyhow!("Audio probe error: {:?}", e))?;
        
        let format = probed.format;
        
        let mut audio = AudioContent::default();
        
        // Get track info
        if let Some(track) = format.tracks().first() {
            audio.sample_rate = track.codec_params.sample_rate.unwrap_or(44100);
            audio.channels = track.codec_params.channels.map(|c| c.count() as u8).unwrap_or(2);
            audio.bit_depth = track.codec_params.bits_per_sample.unwrap_or(16) as u8;
            
            if let Some(n_frames) = track.codec_params.n_frames {
                audio.duration_secs = n_frames as f64 / audio.sample_rate as f64;
            }
        }
        
        // Get metadata
        if let Some(metadata) = format.metadata().current() {
            for tag in metadata.tags() {
                match tag.std_key {
                    Some(symphonia::core::meta::StandardTagKey::TrackTitle) => {
                        audio.title = Some(tag.value.to_string());
                    }
                    Some(symphonia::core::meta::StandardTagKey::Artist) => {
                        audio.artist = Some(tag.value.to_string());
                    }
                    Some(symphonia::core::meta::StandardTagKey::Album) => {
                        audio.album = Some(tag.value.to_string());
                    }
                    Some(symphonia::core::meta::StandardTagKey::Genre) => {
                        audio.genre = Some(tag.value.to_string());
                    }
                    _ => {}
                }
            }
        }
        
        audio.format = path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_uppercase())
            .unwrap_or_else(|| "AUDIO".into());
        
        Ok(FileContent::Audio(audio))
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
        
        // Try to parse MP4 metadata
        if let Ok(context) = mp4parse::read_mp4(&mut std::io::Cursor::new(&data)) {
            if let Some(track) = context.tracks.iter().find(|t| t.track_type == mp4parse::TrackType::Video) {
                if let Some(tkhd) = &track.tkhd {
                    video.width = tkhd.width as u32;
                    video.height = tkhd.height as u32;
                }
                
                if let Some(duration) = track.duration {
                    if let Some(timescale) = track.timescale {
                        video.duration_secs = duration.0 as f64 / timescale.0 as f64;
                    }
                }
            }
        }
        
        Ok(FileContent::Video(video))
    }
    
    fn load_ebook(&self, path: &Path) -> Result<FileContent> {
        let ext = path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
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
            title: doc.mdata("title").unwrap_or_default(),
            author: doc.mdata("creator").unwrap_or_default(),
            publisher: doc.mdata("publisher"),
            language: doc.mdata("language"),
            ..Default::default()
        };
        
        // Get cover image
        if let Some(cover_id) = doc.get_cover_id() {
            if let Some((data, _mime)) = doc.get_resource(&cover_id) {
                ebook.cover_image = Some(data);
            }
        }
        
        // Build TOC
        for (title, href) in doc.toc.iter() {
            ebook.toc.push(TocEntry {
                title: title.clone(),
                href: href.clone(),
                level: 0,
            });
        }
        
        // Get chapter content
        let spine = doc.spine.clone();
        for chapter_id in &spine {
            if let Some((content, _)) = doc.get_resource(chapter_id) {
                if let Ok(html) = String::from_utf8(content) {
                    ebook.chapters.push(EbookChapter {
                        title: Some(chapter_id.clone()),
                        content: html,
                        images: Vec::new(),
                    });
                }
            }
        }
        
        Ok(FileContent::Ebook(ebook))
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
            .map(|e| e.to_lowercase())
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
