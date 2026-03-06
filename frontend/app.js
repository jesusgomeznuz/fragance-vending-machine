const productGrid  = document.getElementById('product-grid');
const payBtn       = document.getElementById('pay-btn');
const statusBox    = document.getElementById('status-box');
const selectionInfo = document.getElementById('selection-info');
const selectedName = document.getElementById('selected-name');
const selectedPrice = document.getElementById('selected-price');
const modeBadge    = document.getElementById('mode-badge');

let selectedProduct = null;

// --- Helpers ---

function setStatus(message, type = 'idle', loading = false) {
  statusBox.className = 'status-box' + (type !== 'idle' ? ` ${type}` : '');
  statusBox.innerHTML = (loading ? '<div class="spinner"></div>' : '') +
    `<p>${message}</p>`;
}

function formatPrice(p) {
  return `$${Number(p).toFixed(2)}`;
}

// --- Status / mode badge ---

async function loadStatus() {
  try {
    const res = await fetch('/status');
    const data = await res.json();
    modeBadge.textContent = data.mode;
    modeBadge.className = 'badge badge--' + data.mode.toLowerCase();
  } catch {
    modeBadge.textContent = 'OFFLINE';
    modeBadge.className = 'badge badge--production';
  }
}

// --- Products ---

async function loadProducts() {
  productGrid.innerHTML = '<p class="loading">Loading products…</p>';
  try {
    const res = await fetch('/products');
    const products = await res.json();
    renderProducts(products);
  } catch {
    productGrid.innerHTML = '<p class="loading">Failed to load products.</p>';
    setStatus('Could not reach the server.', 'error');
  }
}

function renderProducts(products) {
  if (!products.length) {
    productGrid.innerHTML = '<p class="loading">No products available.</p>';
    return;
  }

  productGrid.innerHTML = products.map(p => `
    <div class="product-card ${p.stock_ml <= 0 ? 'out-of-stock' : ''}"
         data-id="${p.id}"
         data-name="${p.name}"
         data-price="${p.price}"
         data-stock="${p.stock_ml}">
      <div class="product-card__name">${p.name}</div>
      <div class="product-card__price">${formatPrice(p.price)}</div>
      <div class="product-card__stock">${p.stock_ml > 0 ? `${p.stock_ml.toFixed(1)} ml` : 'Out of stock'}</div>
    </div>
  `).join('');

  productGrid.querySelectorAll('.product-card:not(.out-of-stock)').forEach(card => {
    card.addEventListener('click', () => selectProduct(card));
  });
}

function selectProduct(card) {
  productGrid.querySelectorAll('.product-card').forEach(c => c.classList.remove('selected'));
  card.classList.add('selected');

  selectedProduct = {
    id:    Number(card.dataset.id),
    name:  card.dataset.name,
    price: Number(card.dataset.price),
  };

  selectedName.textContent = selectedProduct.name;
  selectedPrice.textContent = formatPrice(selectedProduct.price);
  selectionInfo.classList.remove('hidden');
  payBtn.disabled = false;
  setStatus(`Selected: ${selectedProduct.name} — ${formatPrice(selectedProduct.price)}`);
}

// --- Pay flow ---

payBtn.addEventListener('click', async () => {
  if (!selectedProduct) return;

  payBtn.disabled = true;
  setStatus('Processing payment…', 'info', true);

  try {
    // 1. Pay
    const payRes = await fetch('/pay', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ product_id: selectedProduct.id }),
    });
    const payData = await payRes.json();

    if (!payData.success) {
      setStatus(payData.message, 'error');
      payBtn.disabled = false;
      return;
    }

    setStatus('Payment accepted! Dispensing…', 'info', true);

    // 2. Dispense
    const dispRes = await fetch('/dispense', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ product_id: selectedProduct.id }),
    });
    const dispData = await dispRes.json();

    if (dispData.success) {
      setStatus(`Enjoy your ${selectedProduct.name}! Please collect your item.`, 'success');
    } else {
      setStatus('Dispensing failed. Please contact support.', 'error');
    }
  } catch {
    setStatus('Network error. Please try again.', 'error');
    payBtn.disabled = false;
    return;
  }

  // Reset selection after a few seconds
  setTimeout(() => {
    selectedProduct = null;
    selectionInfo.classList.add('hidden');
    payBtn.disabled = true;
    setStatus('Welcome! Select a fragrance to get started.');
    loadProducts(); // refresh stock counts
  }, 4000);
});

// --- Init ---
loadStatus();
loadProducts();
