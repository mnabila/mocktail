// Sidebar show/hide + resizable width, persisted as JSON in localStorage.

const SB_KEY = 'mocktail.sidebar';

export interface SidebarState {
  open: boolean;
  width: number;
  resizing: boolean;
  mobileNav: boolean;
  openCollections: boolean;
}

export const sidebarState = $state<SidebarState>({
  open: true,
  width: 256,
  resizing: false,
  mobileNav: false,
  openCollections: true
});

export function initSidebar(): void {
  try {
    const s = JSON.parse(localStorage.getItem(SB_KEY) || '{}');
    if (typeof s.width === 'number') sidebarState.width = Math.min(480, Math.max(200, s.width));
    if (typeof s.open === 'boolean') sidebarState.open = s.open;
  } catch {
    /* ignore */
  }
}

export function persistSidebar(): void {
  try {
    localStorage.setItem(
      SB_KEY,
      JSON.stringify({ width: sidebarState.width, open: sidebarState.open })
    );
  } catch {
    /* ignore */
  }
}

export function toggleSidebar(): void {
  if (typeof window !== 'undefined' && window.innerWidth < 768) {
    sidebarState.mobileNav = !sidebarState.mobileNav;
    return;
  }
  sidebarState.open = !sidebarState.open;
  persistSidebar();
}

export function closeMobileNav(): void {
  sidebarState.mobileNav = false;
}

function onResize(e: MouseEvent): void {
  if (!sidebarState.resizing) return;
  sidebarState.width = Math.min(480, Math.max(200, e.clientX));
}

function stopResize(): void {
  if (!sidebarState.resizing) return;
  sidebarState.resizing = false;
  document.body.style.userSelect = '';
  window.removeEventListener('mousemove', onResize);
  window.removeEventListener('mouseup', stopResize);
  persistSidebar();
}

export function startSidebarResize(e: MouseEvent): void {
  sidebarState.resizing = true;
  e.preventDefault();
  document.body.style.userSelect = 'none';
  window.addEventListener('mousemove', onResize);
  window.addEventListener('mouseup', stopResize);
}
