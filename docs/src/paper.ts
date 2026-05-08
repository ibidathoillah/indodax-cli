// TypeScript version of paper trading interface
import { PaperTrader } from '../pkg/paper_wasm.js';

// Global variables
let trader: PaperTrader | null = null;
let currentPair = 'btc_idr';
let tradeType = 'buy';
let marketData: any = {};
let priceHistory: any = {};
let placingOrder = false;
let ws: WebSocket | null = null;
let useSimulated = false;

// PAIRS and ASSET_ORDER would be imported or defined here
const PAIRS = [
    { id: 'btc_idr', symbol: 'BTC', icon: '₿' },
    { id: 'eth_idr', symbol: 'ETH', icon: 'Ξ' },
    // Add more pairs as needed
];

const ASSET_ORDER = ['idr', 'usdt', 'btc', 'eth'];

// Initialize
window.init = async function() {
    try {
        const wasm = await import('../pkg/paper_wasm.js');
        await wasm.default();
        const { PaperTrader } = wasm;
        trader = new PaperTrader();

        // Verify trader has default balances
        let balances = trader.get_balances();
        console.log('Initial balances after constructor:', JSON.stringify(balances));

        // If no balances, call init to set defaults
        if (!balances || Object.keys(balances).length === 0) {
            console.log('No balances found, calling init()');
            trader.init();
            balances = trader.get_balances();
            console.log('Balances after init:', JSON.stringify(balances));
        }

        const saved = localStorage.getItem('paper-trading-state');
        if (saved) {
            try {
                console.log('Loading saved state:', saved.substring(0, 100) + '...');
                trader.load_state(saved);
                balances = trader.get_balances();
                console.log('Balances after load:', JSON.stringify(balances));
            } catch(e) {
                console.warn('Failed to load saved state, using defaults:', e);
                trader.init();
            }
        }

        // Final check
        balances = trader.get_balances();
        console.log('Final balances:', JSON.stringify(balances));
        if (!balances || Object.keys(balances).length === 0) {
            console.error('ERROR: No balances available!');
            const balancesDiv = document.getElementById('balances');
            if (balancesDiv) {
                balancesDiv.innerHTML = '<div style="color:var(--error);padding:12px">Error: No balances. <button id="btn-init-balances">Initialize</button></div>';
                const btn = document.getElementById('btn-init-balances');
                if (btn) {
                    btn.addEventListener('click', reinitTrader);
                }
            }
        }

        renderPairSelectors();

        const loadingDiv = document.getElementById('loading');
        const appDiv = document.getElementById('app');
        if (loadingDiv) loadingDiv.style.display = 'none';
        if (appDiv) appDiv.style.display = 'block';

        render();
        startFillChecks();

        // Start data fetching
        fetchTickerAll().catch(e => console.warn('Fetch failed:', e));
        connectWebSocket();
    } catch (e) {
        console.error('Initialization error:', e);
        const loadingDiv = document.getElementById('loading');
        if (loadingDiv) {
            loadingDiv.innerHTML = '<div class="alert alert-error">WASM load failed: ' + (e as Error).message + '</div>';
        }
    }
};

// Reinitialize trader
window.reinitTrader = function() {
    if (!trader) {
        showAlert('Trader not initialized', 'error');
        return;
    }
    try {
        trader.init();
        saveState();
        render();
        showAlert('Trader reinitialized with default balances');
    } catch(e) {
        console.error('Reinit error:', e);
        showAlert('Failed to reinitialize: ' + e, 'error');
    }
};

// Topup balance
window.topupBalance = function() {
    if (!trader) {
        showAlert('Trader not initialized. Please refresh the page.', 'error');
        return;
    }
    const currencyEl = document.getElementById('topup-currency') as HTMLSelectElement;
    const amountEl = document.getElementById('topup-amount') as HTMLInputElement;
    
    if (!currencyEl || !amountEl) {
        showAlert('Form elements not found', 'error');
        return;
    }
    
    const currency = currencyEl.value.toLowerCase();
    const amount = parseFloat(amountEl.value);
    
    if (!currency || isNaN(amount) || amount <= 0) {
        showAlert('Please enter a valid currency and amount', 'error');
        return;
    }
    
    try {
        const result = trader.topup(currency, amount);
        console.log('Topup returned:', JSON.stringify(result));
        const balances = trader.get_balances();
        console.log('Balances after topup:', JSON.stringify(balances));
        
        if (balances && balances[currency]) {
            showAlert(`Added ${amount} to ${currency.toUpperCase()}. New balance: ${parseFloat(balances[currency]).toFixed(2)}`);
        } else {
            showAlert(`Added ${amount} to ${currency.toUpperCase()}. Check balances above.`);
        }
        
        saveState();
        render();
        amountEl.value = '';
    } catch (e) {
        console.error('Topup error:', e);
        showAlert('Topup failed: ' + (e as Error).message || e, 'error');
    }
};

// Test balances
window.testBalances = function() {
    if (!trader) {
        showAlert('Trader not initialized', 'error');
        return;
    }
    try {
        const balances = trader.get_balances();
        console.log('Balances:', JSON.stringify(balances));
        if (balances && typeof balances === 'object') {
            const keys = Object.keys(balances);
            if (keys.length > 0) {
                showAlert('Balances: ' + JSON.stringify(balances).substring(0, 100));
            } else {
                showAlert('No balances found. Calling init()...', 'error');
                trader.init();
                setTimeout(() => {
                    const b2 = trader!.get_balances();
                    showAlert('After init: ' + JSON.stringify(b2).substring(0, 100));
                    render();
                }, 500);
            }
        } else {
            showAlert('Invalid balances object', 'error');
        }
    } catch(e) {
        console.error('Test error:', e);
        showAlert('Error: ' + e, 'error');
    }
};

// Save state
function saveState() {
    if (!trader) return;
    try {
        localStorage.setItem('paper-trading-state', trader.save_state());
    } catch (e) {
        console.warn('Failed to save state:', e);
    }
}

// Render function
function render() {
    renderStats();
    updateQuickPrices();
    renderOrders();
}

// Update quick prices/balances display
function updateQuickPrices() {
    if (!trader) {
        const balancesDiv = document.getElementById('balances');
        if (balancesDiv) {
            balancesDiv.innerHTML = '<div style="color:var(--error);text-align:center;padding:12px">Trader not initialized</div>';
        }
        return;
    }
    
    const b = trader.get_balances();
    console.log('updateQuickPrices - balances:', JSON.stringify(b));
    
    if (!b || (typeof b === 'object' && Object.keys(b).length === 0)) {
        console.warn('No balances, calling init()');
        trader.init();
        const b2 = trader.get_balances();
        if (!b2 || Object.keys(b2).length === 0) {
            const balancesDiv = document.getElementById('balances');
            if (balancesDiv) {
                balancesDiv.innerHTML = '<div style="color:var(--error);text-align:center;padding:12px">No balances. <button id="btn-init-quick">Initialize</button></div>';
                const btn = document.getElementById('btn-init-quick');
                if (btn) btn.addEventListener('click', reinitTrader);
            }
            return;
        }
    }
    
    // Render balances HTML here
    // ... (implementation details)
}

// Placeholder functions that would be implemented
function renderStats() { /* implementation */ }
function renderOrders() { /* implementation */ }
function renderPairSelectors() { /* implementation */ }
function showAlert(msg: string, type = 'success') {
    const alertDiv = document.getElementById('alert');
    if (alertDiv) {
        alertDiv.innerHTML = `<div class="alert alert-${type}">${msg}</div>`;
        setTimeout(() => { alertDiv.innerHTML = ''; }, 5000);
    }
}
function fetchTickerAll() { return Promise.resolve(); }
function connectWebSocket() { }
function startFillChecks() { }

// Initialize event listeners when DOM is loaded
document.addEventListener('DOMContentLoaded', () => {
    // Topup button
    const btnTopup = document.getElementById('btn-topup');
    if (btnTopup) btnTopup.addEventListener('click', () => window.topupBalance!());
    
    // Test balances button
    const btnTest = document.getElementById('btn-test-balances');
    if (btnTest) btnTest.addEventListener('click', () => window.testBalances!());
    
    // Initialize buttons
    const btnInit1 = document.getElementById('btn-init-trader');
    if (btnInit1) btnInit1.addEventListener('click', () => window.reinitTrader!());
    
    const btnInit2 = document.getElementById('btn-init-balances');
    if (btnInit2) btnInit2.addEventListener('click', () => window.reinitTrader!());
    
    // Other buttons (place order, buy/sell type, etc.)
    const btnPlaceOrder = document.getElementById('btn-place-order');
    if (btnPlaceOrder) btnPlaceOrder.addEventListener('click', placeOrder);
    
    const btnBuy = document.getElementById('btn-buy');
    if (btnBuy) btnBuy.addEventListener('click', () => setTradeType('buy'));
    
    const btnSell = document.getElementById('btn-sell');
    if (btnSell) btnSell.addEventListener('click', () => setTradeType('sell'));
});

// Placeholder functions for buttons
function placeOrder() { console.log('Place order clicked'); }
function setTradeType(type: string) { 
    tradeType = type; 
    console.log('Trade type set to:', type);
}
function setPair(pairId: string) { 
    currentPair = pairId; 
    console.log('Pair set to:', pairId);
}
function resetTrading() {
    if (confirm('Reset all trading data?')) {
        trader?.reset();
        localStorage.removeItem('paper-trading-state');
        render();
        showAlert('Trading data reset');
    }
}
function cancelOrder(orderId: number) {
    if (!trader) return;
    try {
        const result = trader.cancel_order(BigInt(orderId));
        if (result && result.success) {
            saveState();
            render();
            showAlert('Order #' + orderId + ' cancelled');
        }
    } catch(e) {
        showAlert('Cancel failed: ' + e, 'error');
    }
}

// Make functions available globally
window.setTradeType = setTradeType;
window.setPair = setPair;
window.resetTrading = resetTrading;
window.cancelOrder = cancelOrder;
