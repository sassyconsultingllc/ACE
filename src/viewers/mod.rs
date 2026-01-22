#![allow(unused_imports)]
//! Viewers module - All file type viewers for Sassy Browser
//! 
//! This module contains specialized viewers for every supported file type:
//! - Images (raster, vector, RAW, PSD)
//! - Documents (DOCX, ODT, RTF)
//! - Spreadsheets (XLSX, CSV, ODS)
//! - PDFs
//! - Chemical/Scientific (PDB, MOL, XYZ, CIF)
//! - Archives (ZIP, RAR, 7z, TAR)
//! - 3D Models (OBJ, STL, GLTF, PLY)
//! - Fonts (TTF, OTF, WOFF)
//! - Audio (MP3, FLAC, WAV, OGG, AAC)
//! - Video (MP4, WebM, AVI, MKV)
//! - eBooks (EPUB, MOBI)
//! - Text/Code (with syntax highlighting)

pub mod archive;
pub mod audio;
pub mod chemical;
pub mod document;
pub mod ebook;
pub mod font;
pub mod image;
pub mod model3d;
pub mod pdf;
pub mod spreadsheet;
pub mod text;
pub mod video;

// Re-export main viewer structs for convenience
pub use archive::ArchiveViewer;
pub use audio::AudioViewer;
pub use chemical::ChemicalViewer;
pub use document::DocumentViewer;
pub use ebook::EbookViewer;
pub use font::FontViewer;
pub use image::ImageViewer;
pub use model3d::Model3DViewer;
pub use pdf::PdfViewer;
pub use spreadsheet::SpreadsheetViewer;
pub use text::TextViewer;
pub use video::VideoViewer;
