export function isLinuxDesktop(): boolean {
  return typeof navigator !== 'undefined' && /Linux/i.test(navigator.userAgent);
}
