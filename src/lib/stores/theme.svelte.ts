import type { Theme } from '$lib/types';

const STORAGE_KEY = 'phantom-ear-theme';

function getInitialTheme(): Theme {
  if (typeof window === 'undefined') return 'dark';
  const stored = localStorage.getItem(STORAGE_KEY);
  if (stored === 'light' || stored === 'dark') return stored;
  return 'dark';
}

function createThemeStore() {
  let theme = $state<Theme>(getInitialTheme());

  function setTheme(newTheme: Theme) {
    theme = newTheme;
    if (typeof window !== 'undefined') {
      localStorage.setItem(STORAGE_KEY, newTheme);
      document.documentElement.setAttribute('data-theme', newTheme);
    }
  }

  function toggleTheme() {
    setTheme(theme === 'dark' ? 'light' : 'dark');
  }

  // Initialize on mount
  if (typeof window !== 'undefined') {
    document.documentElement.setAttribute('data-theme', theme);
  }

  return {
    get theme() { return theme; },
    setTheme,
    toggleTheme,
  };
}

export const themeStore = createThemeStore();
