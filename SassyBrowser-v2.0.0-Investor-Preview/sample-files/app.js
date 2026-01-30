// Sassy Browser Demo - JavaScript Sample
// Demonstrates syntax highlighting

class FileManager {
    constructor() {
        this.files = new Map();
        this.supportedFormats = [
            'pdf', 'docx', 'xlsx', 'pptx', 'csv',
            'png', 'jpg', 'svg', 'webp', 'gif',
            'pdb', 'mol', 'fasta', 'sdf'
        ];
    }

    async loadFile(path) {
        try {
            const response = await fetch(path);
            const data = await response.arrayBuffer();
            this.files.set(path, data);
            console.log(`Loaded: ${path} (${data.byteLength} bytes)`);
            return data;
        } catch (error) {
            console.error(`Failed to load ${path}:`, error);
            throw error;
        }
    }

    isSupported(filename) {
        const ext = filename.split('.').pop()?.toLowerCase();
        return this.supportedFormats.includes(ext);
    }
}

// Initialize on page load
document.addEventListener('DOMContentLoaded', () => {
    const manager = new FileManager();
    console.log('Sassy Browser initialized');
    console.log(`Supporting ${manager.supportedFormats.length} formats`);
});
