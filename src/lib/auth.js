// src/lib/auth.js
import { writable } from 'svelte/store';
import { browser } from '$app/environment';


function loadCookie() {
  if (!browser) return null;
  const saved = localStorage.getItem('session');
  if (!saved) return null;
  try {
    return JSON.parse(saved);
  } catch {
    localStorage.removeItem('session');
    return null;
  }
}

const initialCookie = loadCookie();

export const cookie = writable(initialCookie);
export const isLoggedIn = writable(initialCookie !== null);

export function login(newCookie) {
  if (browser) {
    localStorage.setItem('session', JSON.stringify(newCookie));
  }
  cookie.set(newCookie);
  isLoggedIn.set(true);
}

export function logout() {
  if (browser) {
    localStorage.removeItem('session');
  }
  cookie.set(null);
  isLoggedIn.set(false);
}