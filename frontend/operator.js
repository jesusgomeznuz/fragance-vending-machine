const inventoryList = document.getElementById('inventory-list');
const modeBadge     = document.getElementById('mode-badge');
const opStatus      = document.getElementById('op-status');
const refreshBtn    = document.getElementById('refresh-btn');

let statusTimer = null;

// ── Drag-to-scroll (unifies mouse on Mac and touch on Linux) ──
//
// Problem: when pointerdown lands on an unfocused input, the browser
// captures the drag for text selection instead of scrolling.
// Fix: prevent default on inputs unless already focused, then scroll
// the page manually on pointermove.
//
(function initDragScroll() {
  let startY = 0, startScroll = 0, dragging = false, scrolled = false;

  function maxScrollY() {
    return document.documentElement.scrollHeight - document.documentElement.clientHeight;
  }

  document.addEventListener('pointerdown', e => {
    if (e.target.matches('input') && document.activeElement !== e.target) {
      e.preventDefault(); // blocks drag-select on unfocused input
    }
    startY      = e.clientY;
    startScroll = document.documentElement.scrollTop;
    dragging    = true;
    scrolled    = false;
  }, { passive: false });

  document.addEventListener('pointermove', e => {
    if (!dragging) return;
    const dy = e.clientY - startY;
    if (Math.abs(dy) > 8) {
      if (!scrolled) {
        scrolled = true;
        // Disable pointer events on inputs while scrolling → no selection highlight
        document.body.classList.add('is-scrolling');
      }
      // Clamp to valid scroll range — prevents bounce/snap to top
      const clamped = Math.max(0, Math.min(maxScrollY(), startScroll - dy));
      document.documentElement.scrollTop = clamped;
    }
  }, { passive: true });

  function endDrag(e) {
    document.body.classList.remove('is-scrolling');
    if (!scrolled && e && e.target && e.target.matches('input')) {
      e.target.focus();
      e.target.select();
    }
    dragging = false;
  }

  document.addEventListener('pointerup',     endDrag);
  document.addEventListener('pointercancel', endDrag);
})();

// --- Toast ---

function showStatus(message, type = 'info') {
  opStatus.textContent = message;
  opStatus.className = `op-status show ${type}`;
  clearTimeout(statusTimer);
  statusTimer = setTimeout(() => {
    opStatus.classList.remove('show');
  }, 3500);
}

// --- Status badge ---

async function loadStatus() {
  try {
    const res  = await fetch('/status');
    const data = await res.json();
    modeBadge.textContent = data.mode;
    modeBadge.className   = 'badge badge--' + data.mode.toLowerCase();
  } catch {
    modeBadge.textContent = 'OFFLINE';
    modeBadge.className   = 'badge badge--production';
  }
}

// --- Inventory ---

async function loadInventory() {
  inventoryList.innerHTML = '<p class="loading">Loading…</p>';
  try {
    const res   = await fetch('/inventory');
    const items = await res.json();
    renderInventory(items);
  } catch {
    inventoryList.innerHTML = '<p class="loading">Failed to load inventory.</p>';
  }
}

function stockValueClass(qty, type) {
  return `op-stock-value ${type}${qty === 0 ? ' zero' : ''}`;
}

function renderInventory(items) {
  if (!items.length) {
    inventoryList.innerHTML = '<p class="loading">No products found.</p>';
    return;
  }

  inventoryList.innerHTML = items.map(item => `
    <div class="op-card" data-id="${item.id}">
      <div class="op-card__top">
        <span class="op-card__name">${item.name}</span>
        <span class="op-card__price">$${item.price.toFixed(2)}</span>
      </div>

      <div class="op-card__stock">
        <div class="op-stock-item">
          <span class="op-stock-label">Warehouse</span>
          <span class="${stockValueClass(item.warehouse_g, 'warehouse')}">${item.warehouse_g.toFixed(1)}g</span>
        </div>
        <div class="op-stock-item">
          <span class="op-stock-label">Machine</span>
          <span class="${stockValueClass(item.machine_g, 'machine')}">${item.machine_g.toFixed(1)}g</span>
        </div>
      </div>

      <div class="op-card__actions">
        <div class="op-action">
          <label>Add to Warehouse</label>
          <div class="op-action-row">
            <input type="number" inputmode="numeric" min="1" step="1" value="100" class="purchase-qty" />
            <button class="btn--purchase" onclick="doPurchase(${item.id}, this)">+ Add</button>
          </div>
        </div>
        <div class="op-action">
          <label>Load into Machine</label>
          <div class="op-action-row">
            <input type="number" inputmode="numeric" min="1" step="1" value="50" class="transfer-qty" />
            <button class="btn--transfer" onclick="doTransfer(${item.id}, this)">Load →</button>
          </div>
        </div>
      </div>
    </div>
  `).join('');
}

// --- Actions ---

async function doPurchase(productId, btn) {
  const card = btn.closest('.op-card');
  const qty = parseFloat(card.querySelector('.purchase-qty').value);

  if (!qty || qty <= 0) {
    showStatus('Enter a valid quantity in grams', 'error');
    return;
  }

  btn.disabled = true;
  try {
    const res  = await fetch('/inventory/purchase', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ product_id: productId, quantity_g: qty }),
    });
    const data = await res.json();

    if (data.success) {
      showStatus(data.message, 'success');
      loadInventory();
    } else {
      showStatus(data.error || 'Purchase failed', 'error');
    }
  } catch {
    showStatus('Network error', 'error');
  } finally {
    btn.disabled = false;
  }
}

async function doTransfer(productId, btn) {
  const card = btn.closest('.op-card');
  const qty = parseFloat(card.querySelector('.transfer-qty').value);

  if (!qty || qty <= 0) {
    showStatus('Enter a valid quantity in grams', 'error');
    return;
  }

  btn.disabled = true;
  try {
    const res  = await fetch('/inventory/transfer', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ product_id: productId, quantity_g: qty }),
    });
    const data = await res.json();

    if (data.success) {
      showStatus(data.message, 'success');
      loadInventory();
    } else {
      showStatus(data.error || 'Transfer failed', 'error');
    }
  } catch {
    showStatus('Network error', 'error');
  } finally {
    btn.disabled = false;
  }
}

// --- Init ---
refreshBtn.addEventListener('click', loadInventory);
loadStatus();
loadInventory();
