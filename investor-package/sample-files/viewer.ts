// Sassy Browser Demo - TypeScript Sample
// Demonstrates type-safe code viewing

interface FileFormat {
  extension: string;
  mimeType: string;
  category: 'document' | 'image' | 'scientific' | 'code' | 'data';
}

interface ViewerConfig {
  darkMode: boolean;
  fontSize: number;
  lineNumbers: boolean;
}

class UniversalViewer {
  private formats: Map<string, FileFormat>;
  private config: ViewerConfig;

  constructor(config: Partial<ViewerConfig> = {}) {
    this.formats = new Map();
    this.config = {
      darkMode: true,
      fontSize: 14,
      lineNumbers: true,
      ...config
    };
    this.registerDefaultFormats();
  }

  private registerDefaultFormats(): void {
    const defaults: FileFormat[] = [
      { extension: 'pdf', mimeType: 'application/pdf', category: 'document' },
      { extension: 'pdb', mimeType: 'chemical/x-pdb', category: 'scientific' },
      { extension: 'mol', mimeType: 'chemical/x-mdl-molfile', category: 'scientific' },
      { extension: 'rs', mimeType: 'text/x-rust', category: 'code' },
    ];
    defaults.forEach(f => this.formats.set(f.extension, f));
  }

  canOpen(filename: string): boolean {
    const ext = filename.split('.').pop()?.toLowerCase() ?? '';
    return this.formats.has(ext);
  }
}

export { UniversalViewer, FileFormat, ViewerConfig };
