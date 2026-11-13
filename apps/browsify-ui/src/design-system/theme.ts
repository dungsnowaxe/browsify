import { writable } from 'svelte/store';
import { attrs } from '@stylexjs/stylex';
import { darkTheme } from './tokens.stylex';

export type Appearance = 'system' | 'light' | 'dark';
export const appearance = writable<Appearance>('system');
const storageKey = 'browsify.appearance.v1';

/** Call before mounting; return cleanup for HMR and app teardown. */
export function initializeTheme() {
  const media = window.matchMedia('(prefers-color-scheme: dark)');
  let mode: Appearance = 'system';
  try {
    const saved = localStorage.getItem(storageKey);
    if (saved === 'light' || saved === 'dark' || saved === 'system') mode = saved;
  } catch {
    /* A blocked storage API must not prevent startup. */
  }
  appearance.set(mode);
  const classes = (attrs(darkTheme).class ?? '').split(' ').filter(Boolean);
  function apply() {
    const dark = mode === 'dark' || (mode === 'system' && media.matches);
    for (const name of classes) document.documentElement.classList.toggle(name, dark);
    document.documentElement.style.colorScheme = dark ? 'dark' : 'light';
  }
  const unsubscribe = appearance.subscribe((value) => {
    mode = value;
    apply();
    try {
      localStorage.setItem(storageKey, value);
    } catch {
      /* Optional persistence. */
    }
  });
  media.addEventListener('change', apply);
  return () => {
    unsubscribe();
    media.removeEventListener('change', apply);
  };
}
