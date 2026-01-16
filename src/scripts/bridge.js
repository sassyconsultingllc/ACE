// Sassy Browser Bridge Script
// Injected into web pages to communicate with the browser

(function() {
    'use strict';
    
    // Send message to browser backend
    function sendToBrowser(msgType, payload) {
        if (window.ipc) {
            window.ipc.postMessage(JSON.stringify({
                msg_type: msgType,
                payload: payload
            }));
        }
    }
    
    // Track title changes
    let lastTitle = document.title;
    const titleObserver = new MutationObserver(function() {
        if (document.title !== lastTitle) {
            lastTitle = document.title;
            sendToBrowser('title_changed', document.title);
        }
    });
    
    // Start observing title
    const titleElement = document.querySelector('title');
    if (titleElement) {
        titleObserver.observe(titleElement, { 
            subtree: true, 
            characterData: true, 
            childList: true 
        });
    }
    
    // Track URL changes (for SPAs)
    let lastUrl = location.href;
    function checkUrlChange() {
        if (location.href !== lastUrl) {
            lastUrl = location.href;
            sendToBrowser('url_changed', location.href);
        }
    }
    
    // Check URL periodically for SPA navigation
    setInterval(checkUrlChange, 500);
    
    // Also listen for popstate (browser back/forward)
    window.addEventListener('popstate', function() {
        setTimeout(checkUrlChange, 0);
    });
    
    // Listen for pushState/replaceState
    const originalPushState = history.pushState;
    const originalReplaceState = history.replaceState;
    
    history.pushState = function() {
        originalPushState.apply(this, arguments);
        checkUrlChange();
    };
    
    history.replaceState = function() {
        originalReplaceState.apply(this, arguments);
        checkUrlChange();
    };
    
    // Load events
    window.addEventListener('load', function() {
        sendToBrowser('load_finished', null);
        sendToBrowser('title_changed', document.title);
        sendToBrowser('url_changed', location.href);
    });
    
    // Before unload (navigation starting)
    window.addEventListener('beforeunload', function() {
        sendToBrowser('load_started', null);
    });
    
    // Initial state
    if (document.readyState === 'complete') {
        sendToBrowser('load_finished', null);
        sendToBrowser('title_changed', document.title);
        sendToBrowser('url_changed', location.href);
    } else {
        sendToBrowser('load_started', null);
    }
    
    // Context menu info (right-click)
    document.addEventListener('contextmenu', function(e) {
        const target = e.target;
        let linkUrl = null;
        let imageUrl = null;
        
        // Check for link
        let el = target;
        while (el && el !== document.body) {
            if (el.tagName === 'A' && el.href) {
                linkUrl = el.href;
                break;
            }
            el = el.parentElement;
        }
        
        // Check for image
        if (target.tagName === 'IMG' && target.src) {
            imageUrl = target.src;
        }
        
        sendToBrowser('context_menu', JSON.stringify({
            x: e.clientX,
            y: e.clientY,
            linkUrl: linkUrl,
            imageUrl: imageUrl,
            selectedText: window.getSelection().toString()
        }));
    });
    
    // Expose API to browser
    window.__sassyBrowser = {
        // Get page info
        getPageInfo: function() {
            return {
                title: document.title,
                url: location.href,
                favicon: getFaviconUrl(),
                description: getMetaContent('description'),
                keywords: getMetaContent('keywords')
            };
        },
        
        // Find and highlight text
        findText: function(query, highlightAll) {
            // Clear previous highlights
            clearHighlights();
            
            if (!query) return 0;
            
            const matches = findAllMatches(query);
            if (highlightAll) {
                highlightMatches(matches);
            }
            return matches.length;
        },
        
        // Scroll to element
        scrollToElement: function(selector) {
            const el = document.querySelector(selector);
            if (el) {
                el.scrollIntoView({ behavior: 'smooth', block: 'center' });
            }
        },
        
        // Get selection
        getSelection: function() {
            return window.getSelection().toString();
        },
        
        // Get all links on page
        getLinks: function() {
            return Array.from(document.querySelectorAll('a[href]'))
                .map(a => ({ href: a.href, text: a.textContent.trim() }));
        },
        
        // Get all images on page
        getImages: function() {
            return Array.from(document.querySelectorAll('img[src]'))
                .map(img => ({ src: img.src, alt: img.alt }));
        }
    };
    
    // Helper functions
    function getFaviconUrl() {
        const link = document.querySelector('link[rel*="icon"]');
        if (link) return link.href;
        return location.origin + '/favicon.ico';
    }
    
    function getMetaContent(name) {
        const meta = document.querySelector(`meta[name="${name}"]`);
        return meta ? meta.content : null;
    }
    
    function findAllMatches(query) {
        const matches = [];
        const walker = document.createTreeWalker(
            document.body,
            NodeFilter.SHOW_TEXT,
            null,
            false
        );
        
        const queryLower = query.toLowerCase();
        let node;
        while (node = walker.nextNode()) {
            const text = node.textContent.toLowerCase();
            let pos = 0;
            while ((pos = text.indexOf(queryLower, pos)) !== -1) {
                matches.push({ node, start: pos, end: pos + query.length });
                pos++;
            }
        }
        
        return matches;
    }
    
    function highlightMatches(matches) {
        // Implementation for highlighting search matches
        // Would need to wrap text nodes with highlight spans
    }
    
    function clearHighlights() {
        document.querySelectorAll('.sassy-highlight').forEach(el => {
            el.replaceWith(el.textContent);
        });
    }
    
    console.log('Sassy Browser bridge loaded');
})();
