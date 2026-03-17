// ── DOM refs ──

const carousel     = document.getElementById('carousel');
const carouselWrap = carousel.parentElement;
const prevBtn      = document.getElementById('carousel-prev');
const nextBtn      = document.getElementById('carousel-next');
const modeBadge    = document.getElementById('mode-badge');
const statusBox    = document.getElementById('status-box');

const panelDetail     = document.getElementById('panel-detail');
const panelPayment    = document.getElementById('panel-payment');
const panelProcessing = document.getElementById('panel-processing');
const panelDispense   = document.getElementById('panel-dispense');

const detailHero    = document.getElementById('detail-hero');
const detailName    = document.getElementById('detail-name');
const detailDesc    = document.getElementById('detail-desc');
const detailPrice   = document.getElementById('detail-price');
const detailStock   = document.getElementById('detail-stock');
const detailBack    = document.getElementById('detail-back');
const detailConfirm = document.getElementById('detail-confirm');

const payName    = document.getElementById('pay-name');
const payPrice   = document.getElementById('pay-price');
const payCard    = document.getElementById('pay-card');
const payCoin    = document.getElementById('pay-coin');
const payBack    = document.getElementById('payment-back');

const processingLabel  = document.getElementById('processing-label');
const processingAmount = document.getElementById('processing-amount');

const dispenseName      = document.getElementById('dispense-name');
const dispenseCountdown = document.getElementById('dispense-countdown');

// ── State ──

let products  = [];
let selected  = null;
let resetTick = null;

// ── Carousel drag-to-scroll + tap-to-select ──
//
// On Mac (mouse), overflow-x scroll containers do NOT respond to mouse drag.
// We implement scrolling manually: drag scrolls, tap selects.
// This also unifies behavior with Linux touch screens.

let dragStartX    = 0;
let dragStartLeft = 0;
let isDragging    = false;
let didDrag       = false;
let dragSlide     = null;

carousel.addEventListener('pointerdown', e => {
  dragStartX    = e.clientX;
  dragStartLeft = carousel.scrollLeft;
  isDragging    = true;
  didDrag       = false;
  dragSlide     = e.target.closest('.product-slide:not(.out-of-stock)') || null;
  carousel.style.scrollBehavior = 'auto'; // 1:1 tracking — no smooth lag
  carouselWrap.classList.add('scrolling');
});

document.addEventListener('pointermove', e => {
  if (!isDragging) return;
  const dx = e.clientX - dragStartX;
  if (Math.abs(dx) > 6) {
    didDrag = true;
    carousel.scrollLeft = dragStartLeft - dx; // moves exactly with finger
  }
}, { passive: true });

function endCarouselDrag() {
  if (!isDragging) return;
  isDragging = false;
  carousel.style.scrollBehavior = '';
  carouselWrap.classList.remove('scrolling');

  if (!didDrag && dragSlide) {
    selected = products[Number(dragSlide.dataset.idx)];
    openDetail(Number(dragSlide.dataset.idx));
  }
  dragSlide = null;
  updateNavButtons();
}

document.addEventListener('pointerup',     endCarouselDrag);
document.addEventListener('pointercancel', endCarouselDrag);

// ── Arrow buttons — scroll by one card width ──

function scrollByCard(dir) {
  const slide = carousel.querySelector('.product-slide');
  if (!slide) return;
  const amount = (slide.offsetWidth + 4 * window.innerWidth / 100) * dir;
  carouselWrap.classList.add('scrolling');
  carousel.scrollBy({ left: amount, behavior: 'smooth' });
}

function updateNavButtons() {
  prevBtn.disabled = carousel.scrollLeft <= 2;
  nextBtn.disabled = carousel.scrollLeft >= carousel.scrollWidth - carousel.clientWidth - 2;
}

// Re-evaluate arrow states as user scrolls (native or via buttons)
carousel.addEventListener('scroll', updateNavButtons, { passive: true });
carousel.addEventListener('scrollend', () => carouselWrap.classList.remove('scrolling'));

// ── Brief occasion descriptions (keyed by product name) ──

const DESCRIPTIONS = {
  'Jean Paul Gaultier': 'Seductivo e intenso — ideal para salir de noche',
  'Dior Sauvage':       'Fresco y poderoso — perfecto para cualquier ocasión',
  'Versace Eros':       'Apasionado y audaz — hecho para una cita especial',
  'Acqua di Gio':       'Acuático y fresco — tu compañero de todos los días',
  'YSL Black Opium':    'Cálido y adictivo — pensado para la noche',
};

// ── Card gradient themes (one per product slot) ──

const THEMES = [
  'linear-gradient(160deg, #2a1a4a 0%, #0f0f14 100%)',
  'linear-gradient(160deg, #3a2208 0%, #0f0f14 100%)',
  'linear-gradient(160deg, #0a3a20 0%, #0f0f14 100%)',
  'linear-gradient(160deg, #1a1a4a 0%, #0f0f14 100%)',
  'linear-gradient(160deg, #3a1010 0%, #0f0f14 100%)',
];

// ── tap() — unified pointer press for buttons (no scroll interference) ──
//
// Uses setPointerCapture so a quick press fires even if pointer drifts slightly.
// Safe for buttons; NOT used on carousel slides (would block native scroll).

function tap(el, fn) {
  let startX = 0, startY = 0, moved = false;

  el.addEventListener('pointerdown', e => {
    startX = e.clientX;
    startY = e.clientY;
    moved  = false;
    el.setPointerCapture(e.pointerId);
  });

  el.addEventListener('pointermove', e => {
    if (Math.abs(e.clientX - startX) > 10 || Math.abs(e.clientY - startY) > 10) {
      moved = true;
    }
  });

  el.addEventListener('pointerup',     e => { if (!moved) fn(e); });
  el.addEventListener('pointercancel', () => { moved = true; });
}

// ── Helpers ──

function setStatus(msg, type = 'idle', loading = false) {
  statusBox.className = 'status-box' + (type !== 'idle' ? ` ${type}` : '');
  statusBox.innerHTML = (loading ? '<div class="spinner"></div>' : '') + `<p>${msg}</p>`;
}

function fmt(p) {
  return `$${Number(p).toFixed(2)}`;
}

const ALL_PANELS = [panelDetail, panelPayment, panelProcessing, panelDispense];

function showPanel(panel) {
  ALL_PANELS.forEach(p => p.classList.remove('active'));
  if (panel) panel.classList.add('active');
}

// ── Status badge ──

async function loadStatus() {
  try {
    const data = await fetch('/status').then(r => r.json());
    modeBadge.textContent = data.mode;
    modeBadge.className   = 'badge badge--' + data.mode.toLowerCase();
  } catch {
    modeBadge.textContent = 'OFFLINE';
    modeBadge.className   = 'badge badge--production';
  }
}

// ── Products / Carousel ──

async function loadProducts() {
  carousel.innerHTML = `<p style="color:#5a5470;font-size:2vw;padding:4vh 0 4vh 2vw">Loading products…</p>`;
  try {
    products = await fetch('/products').then(r => r.json());
    renderCarousel();
  } catch {
    carousel.innerHTML = `<p style="color:#5a5470;font-size:2vw;padding:4vh 0 4vh 2vw">Could not load products.</p>`;
    setStatus('Could not reach the server.', 'error');
  }
}

function renderCarousel() {
  carousel.innerHTML = products.map((p, i) => `
    <div class="product-slide ${p.stock_g <= 0 ? 'out-of-stock' : ''}"
         data-idx="${i}"
         style="background-color: #0f0f14;
                background-image: linear-gradient(to bottom, rgba(10,8,18,0.25) 0%, rgba(10,8,18,0.72) 100%), url('/images/${p.id}.png');
                background-size: cover;
                background-position: center;">
      <div class="slide__top">
        <div class="slide__name">${p.name}</div>
      </div>
    </div>
  `).join('');

  updateNavButtons();
}

// ── Detail panel ──

function openDetail(idx) {
  const p = products[idx];

  detailHero.style.backgroundImage    = `linear-gradient(to bottom, rgba(10,8,18,0.15) 0%, rgba(10,8,18,0.88) 100%), url('/images/${p.id}.png')`;
  detailHero.style.backgroundSize     = 'cover';
  detailHero.style.backgroundPosition = 'center';
  detailName.textContent  = p.name;
  detailDesc.textContent  = DESCRIPTIONS[p.name] || '';
  detailPrice.textContent = fmt(p.price);
  detailStock.textContent = p.stock_g > 0
    ? `${p.stock_g.toFixed(1)}g in machine`
    : 'Out of stock';

  showPanel(panelDetail);
  setStatus(`${p.name} — ${fmt(p.price)}`);
}

tap(detailBack, () => {
  showPanel(null);
  setStatus('Welcome! Select a fragrance to get started.');
});

tap(detailConfirm, () => {
  payName.textContent  = selected.name;
  payPrice.textContent = fmt(selected.price);
  showPanel(panelPayment);
  setStatus('Select payment method.');
});

// ── Payment method panel ──

tap(payBack, () => {
  stopPolling();
  const idx = products.findIndex(p => p.id === selected.id);
  openDetail(idx);
});

function startPayment(method) {
  processingLabel.textContent  = method === 'card'
    ? 'Procesando pago con tarjeta…'
    : 'Inserta efectivo y presiona OK…';
  processingAmount.textContent = fmt(selected.price);
  showPanel(panelProcessing);
  setStatus('Procesando pago…', 'info', true);

  completePayment();
}

tap(payCard, () => startPayment('card'));
tap(payCoin, () => startPayment('coin'));

// ── Payment + dispense ──

let pollInterval = null;

function stopPolling() {
  if (pollInterval) {
    clearInterval(pollInterval);
    pollInterval = null;
  }
}

async function completePayment() {
  try {
    const payData = await fetch('/pay', {
      method:  'POST',
      headers: { 'Content-Type': 'application/json' },
      body:    JSON.stringify({ product_id: selected.id }),
    }).then(r => r.json());

    // --- Pago pendiente en terminal MP ---
    if (payData.pending && payData.order_id) {
      processingLabel.textContent = '📱 Acerca tu tarjeta o chip a la terminal';
      setStatus('Esperando pago en terminal…', 'info', true);
      startPolling(payData.order_id, selected.id);
      return;
    }

    // --- Simulación: pago inmediato ---
    if (!payData.success) {
      showPanel(null);
      setStatus(payData.message || 'Pago fallido.', 'error');
      return;
    }

    await doDispense();
  } catch {
    showPanel(null);
    setStatus('Error de red. Intenta de nuevo.', 'error');
  }
}

function startPolling(orderId, productId) {
  stopPolling();
  pollInterval = setInterval(async () => {
    try {
      const data = await fetch(`/payment/${orderId}?product_id=${productId}`)
        .then(r => r.json());

      const status = data.status;

      if (status === 'at_terminal') {
        processingLabel.textContent = '💳 Procesando en terminal…';
      } else if (status === 'processed') {
        stopPolling();
        await doDispense();
      } else if (status === 'failed' || status === 'expired' || status === 'canceled') {
        stopPolling();
        showPanel(null);
        const msg = status === 'expired'  ? 'Tiempo de pago agotado. Intenta de nuevo.'
                  : status === 'canceled' ? 'Pago cancelado.'
                  :                         'Pago rechazado. Intenta con otra tarjeta.';
        setStatus(msg, 'error');
      }
    } catch {
      // Error de red temporal — seguir intentando
    }
  }, 2000);
}

async function doDispense() {
  processingLabel.textContent = 'Dispensando…';
  try {
    const dispData = await fetch('/dispense', {
      method:  'POST',
      headers: { 'Content-Type': 'application/json' },
      body:    JSON.stringify({ product_id: selected.id }),
    }).then(r => r.json());

    if (dispData.success) {
      openDispense();
    } else {
      showPanel(null);
      setStatus('Error al dispensar. Contacta al operador.', 'error');
    }
  } catch {
    showPanel(null);
    setStatus('Error de red al dispensar.', 'error');
  }
}

// ── Dispense panel ──

function openDispense() {
  dispenseName.textContent = selected.name;
  showPanel(panelDispense);
  setStatus(`Enjoy your ${selected.name}!`, 'success');
  startCountdown(5);
}

function startCountdown(secs) {
  clearInterval(resetTick);
  let remaining = secs;
  dispenseCountdown.textContent = `Returning to menu in ${remaining}s`;

  resetTick = setInterval(() => {
    remaining--;
    if (remaining <= 0) {
      clearInterval(resetTick);
      resetToMenu();
    } else {
      dispenseCountdown.textContent = `Returning to menu in ${remaining}s`;
    }
  }, 1000);
}

function resetToMenu() {
  selected = null;
  showPanel(null);
  setStatus('Welcome! Select a fragrance to get started.');
  loadProducts();
}

// ── Carousel arrow buttons ──

tap(prevBtn, () => scrollByCard(-1));
tap(nextBtn, () => scrollByCard(+1));

// ── Service zone → operator panel ──
// DEV: instant tap. Before production: restore hold timer + PIN.

tap(document.getElementById('service-zone'), () => {
  window.location.href = '/operator.html';
});

// ── Init ──

loadStatus();
loadProducts();
