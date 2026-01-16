// Sample background script
console.log('[Sassy Extension] Background script started');

// Example: Track tab count
let tabCount = 0;

// Listen for messages from content scripts
browser.runtime.onMessage.addListener((message, sender, sendResponse) => {
    console.log('[Sassy Extension] Received message:', message);
    
    if (message.action === 'getTabCount') {
        sendResponse({ count: tabCount });
    }
    
    return true;
});

// Example: Set badge text
browser.browserAction?.setBadgeText?.({ text: '🦊' });
