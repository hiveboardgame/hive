// PWA Registration Script
const root = document.documentElement;
const isStandalone = () =>
  window.matchMedia('(display-mode: standalone)').matches ||
  window.navigator.standalone === true;

let appWidth = 0;
let appHeight = 0;
let appOuterHeight = 0;
let lastEditableFocusAt = -Infinity;
let standaloneListenersAttached = false;

const SOFTWARE_KEYBOARD_RESIZE_WINDOW_MS = 1500;
const OUTER_HEIGHT_EPSILON_PX = 2;

const isEditableElement = (element) =>
  element instanceof HTMLTextAreaElement ||
  (element instanceof HTMLInputElement &&
    !['button', 'checkbox', 'color', 'file', 'hidden', 'image', 'radio', 'range', 'reset', 'submit'].includes(element.type)) ||
  (element instanceof HTMLElement && element.isContentEditable);

const isTouchInputDevice = () =>
  window.matchMedia('(pointer: coarse)').matches || navigator.maxTouchPoints > 0;

const wasEditableFocusedRecently = () =>
  performance.now() - lastEditableFocusAt <= SOFTWARE_KEYBOARD_RESIZE_WINDOW_MS;

const isLikelySoftwareKeyboardResize = ({ nextWidth, nextHeight, nextOuterHeight }) => {
  if (appHeight === 0 || nextWidth !== appWidth || nextHeight >= appHeight) {
    return false;
  }

  if (!isTouchInputDevice() || !isEditableElement(document.activeElement) || !wasEditableFocusedRecently()) {
    return false;
  }

  const virtualKeyboardHeight = navigator.virtualKeyboard?.boundingRect?.height ?? 0;
  if (virtualKeyboardHeight > 0) {
    return true;
  }

  // Real app-window resizes change the outer frame height too; keyboard resizes
  // only shrink the inner viewport inside the existing app window.
  if (appOuterHeight !== 0 && nextOuterHeight !== 0) {
    return Math.abs(nextOuterHeight - appOuterHeight) <= OUTER_HEIGHT_EPSILON_PX;
  }

  const viewport = window.visualViewport;
  if (!viewport) {
    return false;
  }

  return Math.round(viewport.height + viewport.offsetTop) < appHeight;
};

const updateViewportHeight = ({ force = false } = {}) => {
  const nextWidth = window.innerWidth;
  const nextHeight = window.innerHeight;
  const nextOuterHeight = window.outerHeight;

  // Installed mobile apps can fire window resizes when the software keyboard
  // appears. Only freeze the height for likely keyboard-driven resizes.
  if (!force && isLikelySoftwareKeyboardResize({ nextWidth, nextHeight, nextOuterHeight })) {
    return;
  }

  appWidth = nextWidth;
  appHeight = nextHeight;
  appOuterHeight = nextOuterHeight;
  root.style.setProperty('--app-height', `${nextHeight}px`);
};

const handleStandaloneFocusIn = (event) => {
  if (isEditableElement(event.target)) {
    lastEditableFocusAt = performance.now();
  }
};

const handleStandaloneResize = () => updateViewportHeight();

const attachStandaloneListeners = () => {
  if (standaloneListenersAttached) {
    return;
  }

  document.addEventListener('focusin', handleStandaloneFocusIn);
  window.addEventListener('resize', handleStandaloneResize);
  standaloneListenersAttached = true;
};

const detachStandaloneListeners = () => {
  if (!standaloneListenersAttached) {
    return;
  }

  document.removeEventListener('focusin', handleStandaloneFocusIn);
  window.removeEventListener('resize', handleStandaloneResize);
  standaloneListenersAttached = false;
  lastEditableFocusAt = -Infinity;
};

const updatePwaLayout = () => {
  const standalone = isStandalone();
  root.dataset.displayMode = standalone ? 'standalone' : 'browser';
  if (standalone) {
    attachStandaloneListeners();
    updateViewportHeight({ force: true });
  } else {
    detachStandaloneListeners();
    appWidth = 0;
    appHeight = 0;
    appOuterHeight = 0;
    root.style.removeProperty('--app-height');
  }
};

updatePwaLayout();
window.addEventListener('pageshow', updatePwaLayout);

if ('serviceWorker' in navigator) {
  window.addEventListener('load', () => {
    navigator.serviceWorker.register('/assets/js/sw.js', { updateViaCache: 'none' })
      .then((registration) => {
        console.log('ServiceWorker registration successful:', registration);
      })
      .catch((error) => {
        console.error('ServiceWorker registration failed:', error);
      });
  });
}
