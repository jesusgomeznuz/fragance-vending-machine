// ── DOM refs ──

const carousel     = document.getElementById('carousel');
const carouselWrap = carousel.parentElement;
const prevBtn      = document.getElementById('carousel-prev');
const nextBtn      = document.getElementById('carousel-next');
const dotsEl       = document.getElementById('dots');
const statusDot    = document.getElementById('status-dot');

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
  carousel.style.scrollBehavior = 'auto';
  carouselWrap.classList.add('scrolling');
});

document.addEventListener('pointermove', e => {
  if (!isDragging) return;
  const dx = e.clientX - dragStartX;
  if (Math.abs(dx) > 6) {
    didDrag = true;
    carousel.scrollLeft = dragStartLeft - dx;
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

// ── Arrow buttons ──

function scrollByCard(dir) {
  const slide = carousel.querySelector('.product-slide');
  if (!slide) return;
  const gap = 5 * window.innerWidth / 100;
  carousel.scrollBy({ left: (slide.offsetWidth + gap) * dir, behavior: 'smooth' });
}

function updateNavButtons() {
  prevBtn.disabled = carousel.scrollLeft <= 2;
  nextBtn.disabled = carousel.scrollLeft >= carousel.scrollWidth - carousel.clientWidth - 2;
  updateDots();
}

carousel.addEventListener('scroll', updateNavButtons, { passive: true });
carousel.addEventListener('scrollend', () => carouselWrap.classList.remove('scrolling'));

// ── Dot indicators ──

function buildDots(count) {
  dotsEl.innerHTML = Array.from({ length: count }, (_, i) =>
    `<span class="dot${i === 0 ? ' active' : ''}"></span>`
  ).join('');
}

function updateDots() {
  const slide = carousel.querySelector('.product-slide');
  if (!slide) return;
  const gap = 5 * window.innerWidth / 100;
  const idx = Math.round(carousel.scrollLeft / (slide.offsetWidth + gap));
  dotsEl.querySelectorAll('.dot').forEach((d, i) =>
    d.classList.toggle('active', i === idx)
  );
}

// ── Descripciones de productos ──

const DESCRIPTIONS = {
  'Jean Paul Gaultier': 'Seductivo e intenso. Ideal para salir de noche.',
  'Dior Sauvage':       'Fresco y poderoso. Perfecto para cualquier ocasión.',
  'Versace Eros':       'Apasionado y audaz. Pensado para una cita especial.',
  'Acqua di Gio':       'Acuático y fresco. Tu compañero de todos los días.',
  'YSL Black Opium':    'Cálido y adictivo. Diseñado para la noche.',
};

// ── tap() — press unificado sin interferir con scroll ──

function tap(el, fn) {
  let startX = 0, startY = 0, moved = false;
  el.addEventListener('pointerdown', e => {
    startX = e.clientX; startY = e.clientY; moved = false;
    el.setPointerCapture(e.pointerId);
  });
  el.addEventListener('pointermove', e => {
    if (Math.abs(e.clientX - startX) > 10 || Math.abs(e.clientY - startY) > 10) moved = true;
  });
  el.addEventListener('pointerup',     e => { if (!moved) fn(e); });
  el.addEventListener('pointercancel', () => { moved = true; });
}

// ── Helpers ──

function fmt(p) { return `$${Number(p).toFixed(2)}`; }

const ALL_PANELS = [panelDetail, panelPayment, panelProcessing, panelDispense];

function showPanel(panel) {
  ALL_PANELS.forEach(p => p.classList.remove('active'));
  if (panel) panel.classList.add('active');
}

// ── Status ──

async function loadStatus() {
  try {
    await fetch('/status').then(r => r.json());
    statusDot.className = 'status-dot online';
  } catch {
    statusDot.className = 'status-dot offline';
  }
}

// ── Productos / Carousel ──

async function loadProducts() {
  carousel.innerHTML = '';
  try {
    products = await fetch('/products').then(r => r.json());
    renderCarousel();
  } catch {
    carousel.innerHTML = `<p style="color:var(--muted);font-size:3.5vw;padding:4vh 8vw">No se pudieron cargar los productos.</p>`;
  }
}

function renderCarousel() {
  carousel.innerHTML = products.map((p, i) => `
    <div class="product-slide ${p.stock_g <= 0 ? 'out-of-stock' : ''}" data-idx="${i}">
      <div class="slide__bg" style="background-image:url('/images/${p.id}.png')"></div>
      <div class="slide__gradient"></div>
      <div class="slide__content">
        <div class="slide__name">${p.name}</div>
        <div class="slide__price">${fmt(p.price)}</div>
        ${p.stock_g <= 0 ? '<div class="slide__oos-label">Sin existencia</div>' : ''}
      </div>
    </div>
  `).join('');

  buildDots(products.length);
  updateNavButtons();
}

// ── Panel: Detalle ──

function openDetail(idx) {
  const p = products[idx];

  // Inyectar imagen de fondo en el hero
  let bg = detailHero.querySelector('.panel-hero__bg');
  if (!bg) {
    bg = document.createElement('div');
    bg.className = 'panel-hero__bg';
    detailHero.insertBefore(bg, detailHero.firstChild);
  }
  bg.style.backgroundImage = `url('/images/${p.id}.png')`;

  detailName.textContent  = p.name;
  detailDesc.textContent  = DESCRIPTIONS[p.name] || '';
  detailPrice.textContent = fmt(p.price);
  detailStock.textContent = p.stock_g > 0
    ? `${p.stock_g.toFixed(1)} g disponibles`
    : 'Sin existencia';

  showPanel(panelDetail);
}

tap(detailBack, () => showPanel(null));

tap(detailConfirm, () => {
  payName.textContent  = selected.name;
  payPrice.textContent = fmt(selected.price);
  showPanel(panelPayment);
});

// ── Panel: Pago ──

tap(payBack, () => {
  stopPolling();
  const idx = products.findIndex(p => p.id === selected.id);
  openDetail(idx);
});

function startPayment(method) {
  processingLabel.textContent  = method === 'card'
    ? 'Acerca tu tarjeta o chip\na la terminal'
    : 'Inserta tu efectivo\ny presiona OK';
  processingAmount.textContent = fmt(selected.price);
  showPanel(panelProcessing);
  completePayment();
}

tap(payCard, () => startPayment('card'));
tap(payCoin, () => startPayment('coin'));

// ── Pago + dispensado ──

let pollInterval = null;

function stopPolling() {
  if (pollInterval) { clearInterval(pollInterval); pollInterval = null; }
}

async function completePayment() {
  try {
    const payData = await fetch('/pay', {
      method:  'POST',
      headers: { 'Content-Type': 'application/json' },
      body:    JSON.stringify({ product_id: selected.id }),
    }).then(r => r.json());

    // Pago pendiente en terminal MP
    if (payData.pending && payData.order_id) {
      processingLabel.textContent = 'Acerca tu tarjeta o chip\na la terminal';
      startPolling(payData.order_id, selected.id);
      return;
    }

    if (!payData.success) {
      showPanel(null);
      return;
    }

    await doDispense();
  } catch {
    showPanel(null);
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
        processingLabel.textContent = 'Procesando en terminal...';
      } else if (status === 'completed') {
        stopPolling();
        await doDispense();
      } else if (status === 'failed' || status === 'expired' || status === 'canceled') {
        stopPolling();
        showPanel(null);
      }
    } catch { /* error de red temporal — seguir intentando */ }
  }, 2000);
}

async function doDispense() {
  processingLabel.textContent = 'Dispensando...';
  try {
    const dispData = await fetch('/dispense', {
      method:  'POST',
      headers: { 'Content-Type': 'application/json' },
      body:    JSON.stringify({ product_id: selected.id }),
    }).then(r => r.json());

    if (dispData.success) openDispense();
    else showPanel(null);
  } catch {
    showPanel(null);
  }
}

// ── Panel: Dispensado ──

function openDispense() {
  dispenseName.textContent = selected.name;
  showPanel(panelDispense);
  startCountdown(6);
}

function startCountdown(secs) {
  clearInterval(resetTick);
  let remaining = secs;
  dispenseCountdown.textContent = `Volviendo al menú en ${remaining}s`;

  resetTick = setInterval(() => {
    remaining--;
    if (remaining <= 0) {
      clearInterval(resetTick);
      resetToMenu();
    } else {
      dispenseCountdown.textContent = `Volviendo al menú en ${remaining}s`;
    }
  }, 1000);
}

function resetToMenu() {
  selected = null;
  showPanel(null);
  loadProducts();
}

// ── Botones de navegación ──

tap(prevBtn, () => scrollByCard(-1));
tap(nextBtn, () => scrollByCard(+1));

// ── Zona de servicio (esquina oculta → panel operador) ──

tap(document.getElementById('service-zone'), () => {
  window.location.href = '/operator.html';
});

// ── Init ──

loadStatus();
loadProducts();
