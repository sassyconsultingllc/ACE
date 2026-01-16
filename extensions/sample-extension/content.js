// Sample content script - runs on every page
console.log('[Sassy Extension] Content script loaded');

// Example: Add a badge to the page
(function() {
    const badge = document.createElement('div');
    badge.id = 'sassy-badge';
    badge.innerHTML = '🦊';
    badge.style.cssText = `
        position: fixed;
        bottom: 20px;
        right: 20px;
        width: 40px;
        height: 40px;
        background: #58a6ff;
        border-radius: 50%;
        display: flex;
        align-items: center;
        justify-content: center;
        font-size: 20px;
        cursor: pointer;
        z-index: 10000;
        box-shadow: 0 2px 8px rgba(0,0,0,0.3);
    `;
    
    badge.addEventListener('click', () => {
        alert('Sassy Browser Extension Active!');
    });
    
    document.body.appendChild(badge);
})();
