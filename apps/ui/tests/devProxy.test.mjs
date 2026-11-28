import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import http from "node:http";
import { test } from "node:test";
import {
  DEV_TOKEN_SENTINEL,
  createBrowsifyDevProxy,
  injectDevSessionBootstrap,
  isLoopbackOrigin,
  parseBrowsifyUiUrl,
} from "../dev-proxy.mjs";

function startServer(handler) {
  return new Promise((resolve, reject) => {
    const server = http.createServer(handler);
    server.on("error", reject);
    server.listen(0, "127.0.0.1", () => {
      const addr = server.address();
      resolve({
        server,
        port: addr.port,
        origin: `http://127.0.0.1:${addr.port}`,
        close: () =>
          new Promise((done) => {
            server.closeAllConnections?.();
            server.close(() => done());
          }),
      });
    });
  });
}

function createMockBrowsifyBackend(token, label) {
  const sseResponses = new Set();
  const receivedRequests = [];

  return {
    receivedRequests,
    pushSse(payload) {
      const frame = `data: ${JSON.stringify(payload)}\n\n`;
      for (const res of sseResponses) {
        res.write(frame);
      }
    },
    start() {
      return startServer((req, res) => {
        const host = req.headers.host ?? "";
        const origin = req.headers.origin ?? "";
        const headerToken = req.headers["x-browsify-token"];
        const hostOk =
          host.startsWith("127.0.0.1:") ||
          host.startsWith("localhost:") ||
          host === "127.0.0.1" ||
          host === "localhost";
        const originOk =
          !origin ||
          origin.startsWith("http://127.0.0.1:") ||
          origin.startsWith("http://localhost:") ||
          origin === "http://127.0.0.1" ||
          origin === "http://localhost";

        // Mirror ui_server.rs strict framing check: Transfer-Encoding is rejected.
        if ("transfer-encoding" in req.headers) {
          res.writeHead(400, { "Content-Type": "application/json" });
          res.end(JSON.stringify({ error: "Transfer-Encoding rejected" }));
          return;
        }

        if (!hostOk || !originOk || headerToken !== token) {
          res.writeHead(403, { "Content-Type": "application/json" });
          res.end(JSON.stringify({ error: "Unauthorized loopback request" }));
          return;
        }

        if (req.method === "GET" && req.url === "/api/events") {
          res.writeHead(200, {
            "Content-Type": "text/event-stream",
            "Cache-Control": "no-cache",
            Connection: "keep-alive",
          });
          res.write(`: connected-${label}\n\n`);
          sseResponses.add(res);
          req.on("close", () => sseResponses.delete(res));
          return;
        }

        if (req.method === "POST" && req.url === "/api/rpc") {
          const chunks = [];
          req.on("data", (c) => chunks.push(c));
          req.on("end", () => {
            const body = Buffer.concat(chunks).toString("utf8");
            receivedRequests.push({
              contentLength: req.headers["content-length"],
              body: JSON.parse(body),
            });
            res.writeHead(200, { "Content-Type": "application/json; charset=utf-8" });
            res.end(
              JSON.stringify([
                { type: "version", message: `v-mock-${label}` },
                { type: "config", message: { configPath: `/tmp/${label}.toml` } },
              ]),
            );
          });
          return;
        }

        res.writeHead(404);
        res.end("Not found");
      });
    },
  };
}

test("parseBrowsifyUiUrl accepts loopback URLs and computes sha256 session", () => {
  const token = "a1b2c3d4e5f60718293a4b5c6d7e8f90";
  const expectedSession = createHash("sha256").update(token).digest("hex");

  const parsed = parseBrowsifyUiUrl(`  "http://127.0.0.1:54321/preferences?token=${token}" `);
  assert.equal(parsed.ok, true);
  assert.deepEqual(parsed.target, {
    origin: "http://127.0.0.1:54321",
    host: "127.0.0.1:54321",
    hostname: "127.0.0.1",
    port: 54321,
    token,
    session: expectedSession,
  });

  const localhost = parseBrowsifyUiUrl(`http://localhost:41234/?token=${token}`);
  assert.equal(localhost.ok, true);
  assert.equal(localhost.target.origin, "http://localhost:41234");
});

test("parseBrowsifyUiUrl rejects non-loopback, non-http, portless, or tokenless URLs", () => {
  const missingToken = parseBrowsifyUiUrl("http://127.0.0.1:54321/rules");
  assert.equal(missingToken.ok, false);
  assert.match(missingToken.error, /\?token=/);

  for (const invalid of [
    "",
    "not a url",
    "https://127.0.0.1:54321/?token=abc",
    "http://192.168.1.10:54321/?token=abc",
    "http://127.0.0.1.evil.test:54321/?token=abc",
    "http://example.com:54321/?token=abc",
    "http://user:pass@127.0.0.1:54321/?token=abc",
    "http://127.0.0.1/?token=abc",
  ]) {
    const result = parseBrowsifyUiUrl(invalid);
    assert.equal(result.ok, false, `expected ${invalid} to be rejected`);
  }
});

test("isLoopbackOrigin allows only local http origins", () => {
  assert.equal(isLoopbackOrigin("http://localhost:5173"), true);
  assert.equal(isLoopbackOrigin("http://127.0.0.1:5173"), true);
  assert.equal(isLoopbackOrigin("https://evil.example"), false);
  assert.equal(isLoopbackOrigin("http://localhost.evil.example:5173"), false);
});

test("injectDevSessionBootstrap injects sha256 session and sentinel, never the raw token", () => {
  const secretToken = "super-secret-session-token-1234";
  const parsed = parseBrowsifyUiUrl(`http://127.0.0.1:54321/?token=${secretToken}`);
  assert.equal(parsed.ok, true);

  const rawHtml = "<!doctype html><html><head><title>Test</title></head><body></body></html>";
  assert.equal(injectDevSessionBootstrap(rawHtml, null), rawHtml);

  const injected = injectDevSessionBootstrap(rawHtml, parsed.target);
  assert.ok(injected.includes(`window.__BROWSIFY_SESSION__ = "${parsed.target.session}"`));
  assert.ok(injected.includes(`window.__BROWSIFY_TOKEN__ = "${DEV_TOKEN_SENTINEL}"`));
  assert.equal(injected.includes(secretToken), false);
});

test("dev proxy forwards RPC and SSE with server-side token and supports live retargeting", async () => {
  const tokenA = "11111111111111111111111111111111";
  const tokenB = "22222222222222222222222222222222";
  const backendA = createMockBrowsifyBackend(tokenA, "alpha");
  const backendB = createMockBrowsifyBackend(tokenB, "beta");

  const srvA = await backendA.start();
  const srvB = await backendB.start();
  const controller = createBrowsifyDevProxy();
  const proxySrv = await startServer((req, res) => {
    controller.middleware(req, res, () => {
      res.writeHead(404);
      res.end("fallthrough");
    });
  });

  try {
    // Starts unconfigured; RPC returns 503.
    const initialStatus = await fetch(`${proxySrv.origin}/__browsify_dev/status`).then((r) =>
      r.json(),
    );
    assert.deepEqual(initialStatus, {
      configured: false,
      targetOrigin: null,
      targetHost: null,
    });

    const unconfiguredRpc = await fetch(`${proxySrv.origin}/api/rpc`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ type: "getInitState" }),
    });
    assert.equal(unconfiguredRpc.status, 503);

    // Rejects cross-origin retarget attempts (CSRF guard).
    const csrfAttempt = await fetch(`${proxySrv.origin}/__browsify_dev/target`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        Origin: "https://evil.example",
      },
      body: JSON.stringify({ url: `${srvA.origin}/?token=${tokenA}` }),
    });
    assert.equal(csrfAttempt.status, 403);

    // Connect to backend A; status never exposes token.
    const setARes = await fetch(`${proxySrv.origin}/__browsify_dev/target`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        Origin: proxySrv.origin,
      },
      body: JSON.stringify({ url: `${srvA.origin}/preferences?token=${tokenA}` }),
    });
    assert.equal(setARes.status, 200);
    const statusA = await setARes.json();
    assert.deepEqual(statusA, {
      configured: true,
      targetOrigin: srvA.origin,
      targetHost: `127.0.0.1:${srvA.port}`,
    });
    assert.equal(JSON.stringify(statusA).includes(tokenA), false);

    // RPC forwards JSON with Content-Length and server-side token even when browser sends placeholder.
    const rpcARes = await fetch(`${proxySrv.origin}/api/rpc?token=${DEV_TOKEN_SENTINEL}`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        "X-Browsify-Token": DEV_TOKEN_SENTINEL,
      },
      body: JSON.stringify({ type: "getInitState" }),
    });
    assert.equal(rpcARes.status, 200);
    const rpcAData = await rpcARes.json();
    assert.equal(rpcAData[0].message, "v-mock-alpha");
    assert.equal(backendA.receivedRequests.length, 1);
    assert.ok(Number(backendA.receivedRequests[0].contentLength) > 0);

    // SSE streams frames from backend A in real time.
    const ssePromise = new Promise((resolve, reject) => {
      const req = http.get(`${proxySrv.origin}/api/events`, (res) => {
        assert.equal(res.statusCode, 200);
        assert.match(res.headers["content-type"] ?? "", /text\/event-stream/);
        let buffer = "";
        res.on("data", (chunk) => {
          buffer += chunk.toString("utf8");
          if (buffer.includes("connected-alpha")) {
            backendA.pushSse({ type: "log", message: "hello-alpha" });
          }
          if (buffer.includes("hello-alpha")) {
            req.destroy();
            resolve(buffer);
          }
        });
      });
      req.on("error", (err) => {
        if (err.code !== "ECONNRESET") reject(err);
      });
    });
    const sseOutput = await ssePromise;
    assert.ok(sseOutput.includes("hello-alpha"));

    // Retarget at runtime to backend B without restarting the proxy.
    const sessionBefore = controller.getTarget()?.session;
    const setBRes = await fetch(`${proxySrv.origin}/__browsify_dev/target`, {
      method: "POST",
      headers: { "Content-Type": "application/json", Origin: proxySrv.origin },
      body: JSON.stringify({ url: `${srvB.origin}/?token=${tokenB}` }),
    });
    assert.equal(setBRes.status, 200);
    assert.notEqual(controller.getTarget()?.session, sessionBefore);

    const rpcBData = await fetch(`${proxySrv.origin}/api/rpc`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ type: "getInitState" }),
    }).then((r) => r.json());
    assert.equal(rpcBData[0].message, "v-mock-beta");

    // Disconnect clears target back to 503.
    const delRes = await fetch(`${proxySrv.origin}/__browsify_dev/target`, {
      method: "DELETE",
      headers: { Origin: proxySrv.origin },
    });
    assert.equal(delRes.status, 200);
    assert.equal(controller.getTarget(), null);
  } finally {
    controller.closeAllStreams();
    await Promise.all([proxySrv.close(), srvA.close(), srvB.close()]);
  }
});
