// Keyboard shortcut utilities for Phantom Ear

export interface KeyboardShortcutOptions {
  ctrl?: boolean;
  shift?: boolean;
  alt?: boolean;
  meta?: boolean;
  preventDefault?: boolean;
}

/**
 * Creates a keyboard event handler for a specific shortcut
 * @param key The key to listen for (e.g., 'r', 'k', 'Enter')
 * @param callback Function to call when shortcut is triggered
 * @param options Modifier keys and options
 */
export function createShortcutHandler(
  key: string,
  callback: () => void,
  options: KeyboardShortcutOptions = {}
): (e: KeyboardEvent) => void {
  const {
    ctrl = false,
    shift = false,
    alt = false,
    meta = false,
    preventDefault = true,
  } = options;

  return (e: KeyboardEvent) => {
    // Check if the key matches (case-insensitive)
    if (e.key.toLowerCase() !== key.toLowerCase()) return;

    // Check modifiers
    const isMac = navigator.platform.toUpperCase().indexOf('MAC') >= 0;
    
    // On Mac, Cmd is used instead of Ctrl for most shortcuts
    const ctrlMatch = isMac ? (ctrl ? e.metaKey : !e.metaKey) : (ctrl ? e.ctrlKey : !e.ctrlKey);
    const shiftMatch = shift ? e.shiftKey : !e.shiftKey;
    const altMatch = alt ? e.altKey : !e.altKey;
    const metaMatch = isMac ? true : (meta ? e.metaKey : !e.metaKey);

    if (ctrlMatch && shiftMatch && altMatch && metaMatch) {
      if (preventDefault) {
        e.preventDefault();
      }
      callback();
    }
  };
}

/**
 * Detects if we're on macOS
 */
export function isMacOS(): boolean {
  return navigator.platform.toUpperCase().indexOf('MAC') >= 0;
}

/**
 * Formats a shortcut for display
 * @param key The key (e.g., 'r')
 * @param modifiers Array of modifiers ('cmd', 'ctrl', 'shift', 'alt')
 */
export function formatShortcut(key: string, modifiers: string[] = []): string {
  const isMac = isMacOS();
  
  const symbols: Record<string, string> = {
    cmd: isMac ? '⌘' : 'Ctrl',
    ctrl: isMac ? '⌃' : 'Ctrl',
    shift: isMac ? '⇧' : 'Shift',
    alt: isMac ? '⌥' : 'Alt',
  };

  const parts = modifiers.map(m => symbols[m] || m);
  parts.push(key.toUpperCase());

  return parts.join(isMac ? '' : '+');
}

/**
 * Common shortcuts used in Phantom Ear
 */
export const SHORTCUTS = {
  toggleRecording: { key: 'r', modifiers: ['cmd', 'shift'], label: 'Toggle Recording' },
  quickSearch: { key: 'k', modifiers: ['cmd'], label: 'Quick Search' },
  toggleSidebar: { key: 'b', modifiers: ['cmd'], label: 'Toggle Sidebar' },
  newMeeting: { key: 'n', modifiers: ['cmd'], label: 'New Meeting' },
  settings: { key: ',', modifiers: ['cmd'], label: 'Settings' },
} as const;
