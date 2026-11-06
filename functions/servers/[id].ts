import {
  resolveServerMetadata,
  buildMetaTagsHtml,
  escapeHtml,
  ServerOgMetadata,
} from "./metadata";

export interface PagesContext<Params = Record<string, string | string[]>> {
  request: Request;
  params: Params;
  next: () => Promise<Response>;
}

/**
 * Fallback HTML transformer for environments without native Cloudflare HTMLRewriter.
 */
export function injectOgTagsIntoHtml(
  html: string,
  metadata: ServerOgMetadata
): string {
  const metaTags = buildMetaTagsHtml(metadata);
  const titleTag = `<title>${escapeHtml(metadata.title)}</title>`;

  // Replace existing <title> if present (use function replacer to prevent special pattern replacement e.g. '$&')
  let transformed = html.replace(/<title>.*?<\/title>/i, () => titleTag);

  // Inject meta tags right before </head>
  if (/<\/head>/i.test(transformed)) {
    transformed = transformed.replace(/<\/head>/i, () => `    ${metaTags}\n  </head>`);
  } else {
    transformed = `${titleTag}\n${metaTags}\n${transformed}`;
  }

  return transformed;
}

/**
 * Cloudflare Pages Functions Handler for /servers/:id
 * Intercepts requests, resolves server metadata, and injects dynamic OpenGraph/Twitter tags.
 */
export const onRequest = async (
  context: PagesContext<{ id?: string }>
): Promise<Response> => {
  const rawId = context.params?.id;
  const url = new URL(context.request.url);
  const baseUrl = `${url.protocol}//${url.host}`;
  const metadata = resolveServerMetadata(rawId, baseUrl);

  // Fetch static asset (SPA index.html)
  const response = await context.next();

  // If Cloudflare HTMLRewriter is available in native runtime
  if (typeof (globalThis as any).HTMLRewriter !== "undefined") {
    return new (globalThis as any).HTMLRewriter()
      .on("title", {
        element(e: any) {
          e.setInnerContent(metadata.title);
        },
      })
      .on("head", {
        element(e: any) {
          e.append(`\n    ${buildMetaTagsHtml(metadata)}\n  `, { html: true });
        },
      })
      .transform(response);
  }

  // Standard fallback
  const originalHtml = await response.text();
  const transformedHtml = await injectOgTagsIntoHtml(originalHtml, metadata);

  const headers = new Headers(response.headers);
  // Remove stale body-length headers that reflect unmodified HTML length
  headers.delete("content-length");
  headers.delete("etag");
  headers.set("content-type", "text/html; charset=utf-8");

  return new Response(transformedHtml, {
    status: response.status,
    headers,
  });
};
