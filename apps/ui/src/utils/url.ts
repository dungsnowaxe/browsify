/** Normalize shorthand URLs for a routing dry run; never opens a destination. */
export function normalizeTestUrl(input: string): string | null {
  const value = input.trim();
  if (!value) return null;
  const hasScheme = /^[a-z][a-z\d+.-]*:\/\//i.test(value) || /^(mailto|tel):/i.test(value);
  try {
    const url = new URL(hasScheme ? value : `https://${value}`);
    // Reject malformed hosts consistently across URL implementations in webviews.
    if (/[\s%]/.test(url.hostname)) return null;
    return url.href;
  } catch {
    return null;
  }
}
