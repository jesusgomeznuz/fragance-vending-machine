const inventoryBody = document.getElementById('inventory-body');
const modeBadge     = document.getElementById('mode-badge');
const opStatus      = document.getElementById('op-status');
const refreshBtn    = document.getElementById('refresh-btn');

let statusTimer = null;

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
  inventoryBody.innerHTML = '<tr><td colspan="6" class="loading">Loading…</td></tr>';
  try {
    const res   = await fetch('/inventory');
    const items = await res.json();
    renderInventory(items);
  } catch {
    inventoryBody.innerHTML = '<tr><td colspan="6" class="loading">Failed to load inventory.</td></tr>';
  }
}

function stockClass(qty, type) {
  return `stock-num ${type}${qty === 0 ? ' zero' : ''}`;
}

function renderInventory(items) {
  if (!items.length) {
    inventoryBody.innerHTML = '<tr><td colspan="6" class="loading">No products found.</td></tr>';
    return;
  }

  inventoryBody.innerHTML = items.map(item => `
    <tr data-id="${item.id}">
      <td class="td-name">${item.name}</td>
      <td class="td-price">$${item.price.toFixed(2)}</td>
      <td><span class="${stockClass(item.warehouse_ml, 'warehouse')}">${item.warehouse_ml.toFixed(1)} ml</span></td>
      <td><span class="${stockClass(item.machine_ml,   'machine')}">${item.machine_ml.toFixed(1)} ml</span></td>

      <td>
        <div class="inline-form">
          <input type="number" min="0.1" step="0.1" value="100" class="purchase-qty" />
          <button class="btn--purchase" onclick="doPurchase(${item.id}, this)">+ Warehouse</button>
        </div>
      </td>

      <td>
        <div class="inline-form">
          <input type="number" min="0.1" step="0.1" value="50" class="transfer-qty" />
          <button class="btn--transfer" onclick="doTransfer(${item.id}, this)">→ Machine</button>
        </div>
      </td>
    </tr>
  `).join('');
}

// --- Actions ---

async function doPurchase(productId, btn) {
  const row = btn.closest('tr');
  const qty = parseFloat(row.querySelector('.purchase-qty').value);

  if (!qty || qty <= 0) {
    showStatus('Enter a valid quantity in ml', 'error');
    return;
  }

  btn.disabled = true;
  try {
    const res  = await fetch('/inventory/purchase', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ product_id: productId, quantity_ml: qty }),
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
  const row = btn.closest('tr');
  const qty = parseFloat(row.querySelector('.transfer-qty').value);

  if (!qty || qty <= 0) {
    showStatus('Enter a valid quantity in ml', 'error');
    return;
  }

  btn.disabled = true;
  try {
    const res  = await fetch('/inventory/transfer', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ product_id: productId, quantity_ml: qty }),
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
