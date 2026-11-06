import { test } from "node:test";
import assert from "node:assert/strict";
import {
  resolveServerMetadata,
  escapeHtml,
  buildMetaTagsHtml,
  DEFAULT_SZZ_METADATA,
} from "./metadata";
import { injectOgTagsIntoHtml, onRequest, PagesContext } from "./[id]";

test("escapeHtml escapes dangerous HTML characters", () => {
  const dangerous = `<script>alert("XSS" & 'attack')</script>`;
  const escaped = escapeHtml(dangerous);
  assert.equal(
    escaped,
    "&lt;script&gt;alert(&quot;XSS&quot; &amp; &#39;attack&#39;)&lt;/script&gt;"
  );
});

test("resolveServerMetadata resolves valid bundled server fixtures", () => {
  const cobblemonMeta = resolveServerMetadata("cobblemon-1-21-1");
  assert.equal(cobblemonMeta.serverId, "cobblemon-1-21-1");
  assert.match(cobblemonMeta.title, /Cobblemon.*szz 마인크래프트/);
  assert.match(cobblemonMeta.description, /1\.21\.1/);
  assert.equal(cobblemonMeta.url, "https://szz.nz/servers/cobblemon-1-21-1");

  const skyfactoryMeta = resolveServerMetadata("skyfactory-4");
  assert.equal(skyfactoryMeta.serverId, "skyfactory-4-1-12-2");
  assert.match(skyfactoryMeta.title, /SkyFactory 4.*szz 마인크래프트/);
  assert.match(skyfactoryMeta.description, /1\.12\.2/);
});

test("resolveServerMetadata guards against XSS and Path Traversal inputs", () => {
  // Malicious path traversal
  const traversal = resolveServerMetadata("../../package.json");
  assert.equal(traversal.serverId, DEFAULT_SZZ_METADATA.serverId);
  assert.equal(traversal.title, DEFAULT_SZZ_METADATA.title);

  // XSS injection attempt
  const xss = resolveServerMetadata("<script>alert(1)</script>");
  assert.equal(xss.serverId, DEFAULT_SZZ_METADATA.serverId);

  // Excessive length input (> 64 chars)
  const excessive = resolveServerMetadata("a".repeat(70));
  assert.equal(excessive.serverId, DEFAULT_SZZ_METADATA.serverId);

  // Non-string or null input
  const nonString = resolveServerMetadata(null as any);
  assert.equal(nonString.serverId, DEFAULT_SZZ_METADATA.serverId);
});

test("resolveServerMetadata falls back to default szz metadata for unknown server", () => {
  const unknown = resolveServerMetadata("non-existent-server-999");
  assert.equal(unknown.serverId, DEFAULT_SZZ_METADATA.serverId);
  assert.equal(unknown.title, DEFAULT_SZZ_METADATA.title);
  assert.equal(unknown.url, "https://szz.nz/servers/non-existent-server-999");
});

test("buildMetaTagsHtml produces valid meta tags", () => {
  const meta = resolveServerMetadata("cobblemon-1-21-1");
  const html = buildMetaTagsHtml(meta);

  assert.match(html, /<meta property="og:title" content="Cobblemon/);
  assert.match(html, /<meta property="og:type" content="website" \/>/);
  assert.match(html, /<meta name="twitter:card" content="summary_large_image" \/>/);
  assert.match(html, /<meta name="theme-color" content="#A7C957" \/>/);
});

test("injectOgTagsIntoHtml injects tags into HTML structure", async () => {
  const sampleHtml = `<!DOCTYPE html>
<html>
  <head>
    <meta charset="UTF-8" />
    <title>SZZ Launcher</title>
  </head>
  <body>
    <div id="root"></div>
  </body>
</html>`;

  const meta = resolveServerMetadata("cobblemon-1-21-1");
  const result = await injectOgTagsIntoHtml(sampleHtml, meta);

  assert.match(result, /<title>Cobblemon.*szz 마인크래프트<\/title>/);
  assert.match(result, /<meta property="og:title"/);
  assert.match(result, /<div id="root"><\/div>/);
});

test("onRequest Cloudflare Pages Function handles request and returns updated HTML", async () => {
  const mockHtml = `<!DOCTYPE html><html><head><title>Old</title></head><body>App</body></html>`;

  const context: PagesContext<{ id?: string }> = {
    request: new Request("https://szz.nz/servers/cobblemon-1-21-1"),
    params: { id: "cobblemon-1-21-1" },
    next: async () =>
      new Response(mockHtml, {
        headers: { "content-type": "text/html" },
      }),
  };

  const response = await onRequest(context);
  assert.equal(response.status, 200);

  const text = await response.text();
  assert.match(text, /Cobblemon.*szz 마인크래프트/);
  assert.match(text, /og:image/);
});
