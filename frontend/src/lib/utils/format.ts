// Shared formatting and OpenAPI metadata extraction helpers.

/** Build a DNS-label-style slug (mirrors backend derivation). */
export function slugify(s: string | null | undefined, fallback = 'item'): string {
  return (
    String(s ?? '')
      .toLowerCase()
      .trim()
      .replace(/[^a-z0-9]+/g, '-')
      .replace(/^-+|-+$/g, '') || fallback
  );
}

/** Build the public mount URL for a collection. */
export function mountUrl(wsSlug: string, collSlug: string, basePath?: string | null): string {
  const proto = typeof location !== 'undefined' ? location.protocol : 'http:';
  const host = typeof location !== 'undefined' ? location.host : 'mocktail';
  return `${proto}//${wsSlug}.${host}/${collSlug}${basePath || ''}`;
}

/** Render a unix-millis timestamp in the browser's own locale + timezone. */
export function fmtTime(
  ms?: number | null,
  { relative = false }: { relative?: boolean } = {}
): string {
  if (!ms) return relative ? '—' : '';
  const d = new Date(ms);
  if (!relative) return d.toLocaleString();
  const diff = Math.round((Date.now() - ms) / 1000);
  const rel =
    diff < 60
      ? `${diff}s ago`
      : diff < 3600
        ? `${Math.round(diff / 60)}m ago`
        : `${Math.round(diff / 3600)}h ago`;
  return `${d.toLocaleString()} · ${rel}`;
}

/** Extract collection name and basePath from an OpenAPI / Swagger JSON or YAML string. */
export function extractSpecMeta(text?: string | null): {
  title: string | null;
  basePath: string | null;
} {
  if (!text?.trim()) return { title: null, basePath: null };
  try {
    const parsed = JSON.parse(text);
    const title = parsed?.info?.title ? String(parsed.info.title).trim() : null;
    const bp = parsed?.basePath || parsed?.servers?.[0]?.url || '';
    const basePath = bp && typeof bp === 'string' && bp.startsWith('/') ? bp : null;
    return { title, basePath };
  } catch {
    const titleMatch = text.match(/^\s*title:\s*["']?([^"'\n#]+)["']?/m);
    const bpMatch = text.match(/^\s*basePath:\s*["']?(\/[^"'\s#]*)["']?/m);
    return {
      title: titleMatch?.[1]?.trim() ?? null,
      basePath: bpMatch?.[1]?.trim() ?? null
    };
  }
}
