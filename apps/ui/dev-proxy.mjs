import { createHash } from "node:crypto";
import http from "node:http";

const MAX_RPC_BODY_BYTES = 1024 * 1024;
const MAX_CONTROL_BODY_BYTES = 8 * 1024;
const UPSTREAM_RPC_TIMEOUT_MS = 10_000;
export const DEV_TOKEN_SENTINEL = "browsify-dev-proxy";

export function isLoopbackOrigin(origin) {
  try {
    const parsed = new URL(origin);
    return (
      parsed.protocol === "http:" &&
      (parsed.hostname === "127.0.0.1" || parsed.hostname === "localhost")
    );
  } catch {
    return false;
  }
}

export function parseBrowsifyUiUrl(raw) {
  const cleaned = String(raw ?? "")
    .trim()
    .replace(/^['"]|['"]$/g, "")
    .trim();
  if (!cleaned) {
    return {
      ok: false,
      error: "Paste a Browsify UI URL such as http://127.0.0.1:54321/?token=…",
    };
  }

  let parsed;
  try {
    parsed = new URL(cleaned);
  } catch {
    return {
      ok: false,
      error: "Invalid URL. Paste the full http://127.0.0.1:<port>/?token=… URL from Browsify.",
    };
  }

  if (parsed.protocol !== "http:") {
    return {
      ok: false,
      error: "Only local http:// Browsify UI URLs are supported.",
    };
  }

  if (parsed.username || parsed.password) {
    return {
      ok: false,
      error: "Credentials in the target URL are not allowed.",
    };
  }

  if (parsed.hostname !== "127.0.0.1" && parsed.hostname !== "localhost") {
    return {
      ok: false,
      error: "Only loopback targets (127.0.0.1 or localhost) are allowed.",
    };
  }

  if (!parsed.port || !/^\d+$/.test(parsed.port)) {
    return {
      ok: false,
      error:
        "Target URL must include Browsify’s local port (e.g. http://127.0.0.1:54321/?token=…).",
    };
  }

  const port = Number(parsed.port);
  if (!Number.isInteger(port) || port < 1 || port > 65535) {
    return {
      ok: false,
      error: "Target port must be between 1 and 65535.",
    };
  }

  const token = parsed.searchParams.get("token")?.trim() ?? "";
  if (!token) {
    return {
      ok: false,
      error:
        "URL is missing ?token=…. Open Preferences from Browsify’s menu bar (or run browsify ui --no-open) to get a fresh URL with its session token.",
    };
  }

  const host = `${parsed.hostname}:${port}`;
  const origin = `http://${host}`;
  const session = createHash("sha256").update(token).digest("hex");

  return {
    ok: true,
    target: {
      origin,
      host,
      hostname: parsed.hostname,
      port,
      token,
      session,
    },
  };
}

function sendJson(res, status, payload) {
  const body = Buffer.from(JSON.stringify(payload), "utf8");
  res.statusCode = status;
  res.setHeader("Content-Type", "application/json; charset=utf-8");
  res.setHeader("Content-Length", String(body.length));
  res.setHeader("Cache-Control", "no-store");
  res.end(body);
}

function readBody(req, limitBytes) {
  return new Promise((resolve, reject) => {
    const chunks = [];
    let size = 0;
    let settled = false;

    const fail = (err) => {
      if (settled) return;
      settled = true;
      reject(err);
    };

    req.on("data", (chunk) => {
      if (settled) return;
      const buf = typeof chunk === "string" ? Buffer.from(chunk) : chunk;
      size += buf.length;
      if (size > limitBytes) {
        fail(new Error("PAYLOAD_TOO_LARGE"));
        req.resume();
        return;
      }
      chunks.push(buf);
    });

    req.on("end", () => {
      if (settled) return;
      settled = true;
      resolve(Buffer.concat(chunks));
    });

    req.on("error", (err) => fail(err));
  });
}

export function injectDevSessionBootstrap(html, target) {
  if (!target) return html;
  const script = `<script>window.__BROWSIFY_SESSION__ = ${JSON.stringify(target.session)};window.__BROWSIFY_TOKEN__ = ${JSON.stringify(DEV_TOKEN_SENTINEL)};</script>`;
  const headClose = html.indexOf("</head>");
  if (headClose !== -1) {
    return `${html.slice(0, headClose)}${script}${html.slice(headClose)}`;
  }
  return `${script}${html}`;
}

export function createBrowsifyDevProxy(initialUrl) {
  let currentTarget = null;
  const activeSseStreams = new Set();

  const closeAllStreams = () => {
    for (const upstream of activeSseStreams) {
      upstream.destroy();
    }
    activeSseStreams.clear();
  };

  const getStatus = () => ({
    configured: currentTarget !== null,
    targetOrigin: currentTarget?.origin ?? null,
    targetHost: currentTarget?.host ?? null,
  });

  const setTarget = (rawUrl) => {
    const parsed = parseBrowsifyUiUrl(rawUrl);
    if (!parsed.ok) return parsed;
    closeAllStreams();
    currentTarget = parsed.target;
    return parsed;
  };

  const clearTarget = () => {
    closeAllStreams();
    currentTarget = null;
  };

  if (initialUrl?.trim()) {
    const parsed = setTarget(initialUrl);
    if (parsed.ok) {
      console.log(
        `[browsify-dev] Proxying /api/* -> ${parsed.target.origin} (session ${parsed.target.session.slice(0, 8)}…)`,
      );
    } else {
      console.warn(`[browsify-dev] Ignoring BROWSIFY_UI_URL: ${parsed.error}`);
    }
  }

  const middleware = (req, res, next) => {
    const rawUrl = req.url ?? "/";
    const pathname = rawUrl.split("?")[0];
    const method = (req.method ?? "GET").toUpperCase();

    if (pathname === "/__browsify_dev/status" && method === "GET") {
      sendJson(res, 200, getStatus());
      return;
    }

    if (pathname === "/__browsify_dev/target") {
      const origin = req.headers.origin;
      if (origin && !isLoopbackOrigin(origin)) {
        sendJson(res, 403, { error: "Forbidden cross-origin dev proxy request" });
        return;
      }

      if (method === "DELETE") {
        clearTarget();
        sendJson(res, 200, getStatus());
        return;
      }

      if (method !== "POST") {
        sendJson(res, 405, { error: "Method not allowed" });
        return;
      }

      void readBody(req, MAX_CONTROL_BODY_BYTES)
        .then((buf) => {
          let payload;
          try {
            payload = JSON.parse(buf.toString("utf8"));
          } catch {
            sendJson(res, 400, { error: "Invalid JSON payload" });
            return;
          }
          const url =
            typeof payload === "object" &&
            payload !== null &&
            typeof payload.url === "string"
              ? payload.url
              : "";
          const result = setTarget(url);
          if (!result.ok) {
            sendJson(res, 400, { error: result.error });
            return;
          }
          sendJson(res, 200, getStatus());
        })
        .catch((err) => {
          const isTooLarge =
            err instanceof Error && err.message === "PAYLOAD_TOO_LARGE";
          sendJson(
            res,
            isTooLarge ? 413 : 400,
            { error: isTooLarge ? "Request body too large" : "Invalid request" },
          );
        });
      return;
    }

    if (pathname === "/api/rpc" && method === "POST") {
      const target = currentTarget;
      if (!target) {
        sendJson(res, 503, {
          error: "No Browsify backend target configured in the Vite dev proxy.",
        });
        return;
      }

      void readBody(req, MAX_RPC_BODY_BYTES)
        .then((body) => {
          const upstream = http.request(
            {
              hostname: target.hostname,
              port: target.port,
              path: "/api/rpc",
              method: "POST",
              timeout: UPSTREAM_RPC_TIMEOUT_MS,
              headers: {
                Host: target.host,
                Origin: target.origin,
                "Content-Type":
                  req.headers["content-type"] ?? "application/json; charset=utf-8",
                "Content-Length": String(body.length),
                "X-Browsify-Token": target.token,
                Connection: "close",
              },
            },
            (upstreamRes) => {
              const chunks = [];
              upstreamRes.on("data", (chunk) => {
                chunks.push(typeof chunk === "string" ? Buffer.from(chunk) : chunk);
              });
              upstreamRes.on("end", () => {
                if (res.writableEnded) return;
                const responseBody = Buffer.concat(chunks);
                res.statusCode = upstreamRes.statusCode ?? 502;
                res.setHeader(
                  "Content-Type",
                  upstreamRes.headers["content-type"] ??
                    "application/json; charset=utf-8",
                );
                res.setHeader("Content-Length", String(responseBody.length));
                res.setHeader("Cache-Control", "no-store");
                res.end(responseBody);
              });
            },
          );

          upstream.on("timeout", () => {
            upstream.destroy(new Error("Upstream Browsify RPC timed out"));
          });

          upstream.on("error", () => {
            if (res.writableEnded) return;
            sendJson(res, 502, {
              error: `Could not reach Browsify at ${target.origin}.`,
            });
          });

          upstream.end(body);
        })
        .catch((err) => {
          if (res.writableEnded) return;
          const isTooLarge =
            err instanceof Error && err.message === "PAYLOAD_TOO_LARGE";
          sendJson(res, isTooLarge ? 413 : 400, {
            error: isTooLarge ? "RPC payload too large" : "Failed to read RPC body",
          });
        });
      return;
    }

    if (pathname === "/api/events" && method === "GET") {
      const target = currentTarget;
      if (!target) {
        sendJson(res, 503, {
          error: "No Browsify backend target configured in the Vite dev proxy.",
        });
        return;
      }

      const upstream = http.request(
        {
          hostname: target.hostname,
          port: target.port,
          path: "/api/events",
          method: "GET",
          headers: {
            Host: target.host,
            Origin: target.origin,
            Accept: "text/event-stream",
            "X-Browsify-Token": target.token,
            Connection: "keep-alive",
          },
        },
        (upstreamRes) => {
          const status = upstreamRes.statusCode ?? 502;
          if (status !== 200) {
            const chunks = [];
            upstreamRes.on("data", (chunk) => {
              chunks.push(typeof chunk === "string" ? Buffer.from(chunk) : chunk);
            });
            upstreamRes.on("end", () => {
              activeSseStreams.delete(upstream);
              if (res.writableEnded) return;
              const body = Buffer.concat(chunks);
              res.statusCode = status;
              res.setHeader(
                "Content-Type",
                upstreamRes.headers["content-type"] ??
                  "application/json; charset=utf-8",
              );
              res.setHeader("Content-Length", String(body.length));
              res.setHeader("Cache-Control", "no-store");
              res.end(body);
            });
            return;
          }

          res.writeHead(200, {
            "Content-Type": "text/event-stream",
            "Cache-Control": "no-cache",
            Connection: "keep-alive",
          });
          res.flushHeaders?.();

          upstreamRes.on("data", (chunk) => {
            if (!res.writableEnded) {
              res.write(chunk);
            }
          });

          upstreamRes.on("end", () => {
            activeSseStreams.delete(upstream);
            if (!res.writableEnded) res.end();
          });

          upstreamRes.on("error", () => {
            activeSseStreams.delete(upstream);
            if (!res.writableEnded) res.end();
          });
        },
      );

      activeSseStreams.add(upstream);

      const teardown = () => {
        activeSseStreams.delete(upstream);
        upstream.destroy();
      };

      req.on("close", teardown);
      res.on("close", teardown);

      upstream.on("error", () => {
        activeSseStreams.delete(upstream);
        if (!res.headersSent) {
          sendJson(res, 502, {
            error: `Could not connect to Browsify event stream at ${target.origin}.`,
          });
        } else if (!res.writableEnded) {
          res.end();
        }
      });

      upstream.end();
      return;
    }

    next();
  };

  return {
    getTarget: () => currentTarget,
    getStatus,
    setTarget,
    clearTarget,
    closeAllStreams,
    middleware,
  };
}

export function browsifyDevProxyPlugin(
  initialUrl = process.env.BROWSIFY_UI_URL,
) {
  let controller = null;

  return {
    name: "browsify-dev-proxy",
    apply: "serve",
    configureServer(server) {
      controller = createBrowsifyDevProxy(initialUrl);
      server.middlewares.use(controller.middleware);
      server.httpServer?.once("close", () => {
        controller?.closeAllStreams();
      });
    },
    transformIndexHtml(html) {
      return injectDevSessionBootstrap(html, controller?.getTarget() ?? null);
    },
  };
}
