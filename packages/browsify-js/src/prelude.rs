pub const JS_PRELUDE: &str = r##"(function() {
globalThis.self = globalThis;
globalThis.module = { exports: undefined };
globalThis.exports = globalThis.module.exports;

globalThis.__browsifyWarnings = [];
globalThis.__browsifyErrors = [];

globalThis.console = {
  log: (...args) => {},
  info: (...args) => {},
  debug: (...args) => {},
  warn: (...args) => {
    globalThis.__browsifyWarnings.push(args.map(String).join(" "));
  },
  error: (...args) => {
    globalThis.__browsifyErrors.push(args.map(String).join(" "));
  },
};

class URLSearchParamsPolyfill {
  constructor(init = "") {
    this._entries = [];
    if (typeof init === "string") {
      const s = init.startsWith("?") ? init.slice(1) : init;
      if (s) {
        for (const pair of s.split("&")) {
          if (!pair) continue;
          const idx = pair.indexOf("=");
          if (idx === -1) {
            this._entries.push([decodeURIComponent(pair.replace(/\+/g, " ")), ""]);
          } else {
            const k = decodeURIComponent(pair.slice(0, idx).replace(/\+/g, " "));
            const v = decodeURIComponent(pair.slice(idx + 1).replace(/\+/g, " "));
            this._entries.push([k, v]);
          }
        }
      }
    } else if (init && typeof init === "object") {
      for (const [k, v] of Object.entries(init)) {
        this._entries.push([String(k), String(v)]);
      }
    }
  }
  append(name, value) {
    this._entries.push([String(name), String(value)]);
    this._notify();
  }
  delete(name) {
    const k = String(name);
    this._entries = this._entries.filter((e) => e[0] !== k);
    this._notify();
  }
  get(name) {
    const k = String(name);
    const found = this._entries.find((e) => e[0] === k);
    return found ? found[1] : null;
  }
  getAll(name) {
    const k = String(name);
    return this._entries.filter((e) => e[0] === k).map((e) => e[1]);
  }
  has(name) {
    const k = String(name);
    return this._entries.some((e) => e[0] === k);
  }
  set(name, value) {
    const k = String(name);
    const v = String(value);
    let replaced = false;
    const next = [];
    for (const entry of this._entries) {
      if (entry[0] === k) {
        if (!replaced) {
          next.push([k, v]);
          replaced = true;
        }
      } else {
        next.push(entry);
      }
    }
    if (!replaced) next.push([k, v]);
    this._entries = next;
    this._notify();
  }
  entries() {
    return this._entries[Symbol.iterator]();
  }
  keys() {
    return this._entries.map((e) => e[0])[Symbol.iterator]();
  }
  values() {
    return this._entries.map((e) => e[1])[Symbol.iterator]();
  }
  forEach(cb) {
    for (const [k, v] of this._entries) cb(v, k, this);
  }
  [Symbol.iterator]() {
    return this.entries();
  }
  toString() {
    return this._entries
      .map(
        ([k, v]) =>
          encodeURIComponent(k).replace(/%20/g, "+") +
          "=" +
          encodeURIComponent(v).replace(/%20/g, "+")
      )
      .join("&");
  }
  _notify() {
    if (this._owner) {
      const s = this.toString();
      this._owner._search = s ? "?" + s : "";
    }
  }
}

function parseRawUrl(input, base) {
  if (typeof globalThis.__browsifyParseUrl === "function") {
    const baseStr = base !== undefined ? String(base) : "";
    const json = globalThis.__browsifyParseUrl(String(input), baseStr);
    if (!json) {
      throw new TypeError(`Invalid URL: ${input}`);
    }
    return JSON.parse(json);
  }
  let str = String(input).trim();
  const m = /^([a-zA-Z][a-zA-Z0-9+.-]*):(?:\/\/(?:([^:@/?#]+)(?::([^@/?#]*))?@)?(\[[^\]]+\]|[^:/?#]+)(?::(\d+))?)?([^?#]*)(?:\?([^#]*))?(?:#(.*))?$/.exec(str);
  if (!m) {
    throw new TypeError(`Invalid URL: ${input}`);
  }
  const scheme = m[1].toLowerCase();
  const hasSlashes = str.slice(scheme.length + 1).startsWith("//");
  if ((scheme === "http" || scheme === "https" || scheme === "ftp") && (!hasSlashes || !m[4])) {
    throw new TypeError(`Invalid URL: ${input}`);
  }
  const username = m[2] ? decodeURIComponent(m[2]) : "";
  const password = m[3] ? decodeURIComponent(m[3]) : "";
  const hostname = m[4] ? m[4] : "";
  const port = m[5] ? m[5] : "";
  let pathname = m[6] || "";
  if ((scheme === "http" || scheme === "https" || scheme === "ftp" || scheme === "ws" || scheme === "wss") && pathname === "") {
    pathname = "/";
  }
  const search = m[7] !== undefined && m[7] !== "" ? "?" + m[7] : "";
  const hash = m[8] !== undefined && m[8] !== "" ? "#" + m[8] : "";
  return {
    scheme,
    hasSlashes,
    username,
    password,
    hostname,
    port,
    pathname,
    search,
    hash,
  };
}

class URLPolyfill {
  constructor(url, base) {
    const p = parseRawUrl(url, base);
    this._scheme = p.scheme;
    this._hasSlashes = p.hasSlashes;
    this.username = p.username;
    this.password = p.password;
    this._hostname = p.hostname;
    this._port = p.port;
    this._pathname = p.pathname;
    this._search = p.search;
    this._hash = p.hash;
    this._searchParams = new URLSearchParamsPolyfill(this._search);
    this._searchParams._owner = this;
  }

  get protocol() {
    return this._scheme + ":";
  }
  set protocol(val) {
    const clean = String(val).replace(/:$/, "").toLowerCase();
    if (clean) this._scheme = clean;
  }

  get hostname() {
    return this._hostname;
  }
  set hostname(val) {
    const s = String(val);
    if (s.startsWith("[")) {
      const closeIdx = s.indexOf("]");
      this._hostname = closeIdx !== -1 ? s.slice(0, closeIdx + 1) : s;
      return;
    }
    const idx = s.indexOf(":");
    if (idx !== -1) {
      this._hostname = s.slice(0, idx);
    } else {
      this._hostname = s;
    }
  }

  get port() {
    return this._port;
  }
  set port(val) {
    this._port = val === null || val === undefined ? "" : String(val);
  }

  get host() {
    return this._port ? `${this._hostname}:${this._port}` : this._hostname;
  }
  set host(val) {
    const s = String(val);
    if (s.startsWith("[")) {
      const closeIdx = s.indexOf("]");
      if (closeIdx !== -1) {
        this._hostname = s.slice(0, closeIdx + 1);
        const rest = s.slice(closeIdx + 1);
        this._port = rest.startsWith(":") ? rest.slice(1) : "";
        return;
      }
    }
    const idx = s.indexOf(":");
    if (idx !== -1) {
      this._hostname = s.slice(0, idx);
      this._port = s.slice(idx + 1);
    } else {
      this._hostname = s;
      this._port = "";
    }
  }

  get pathname() {
    return this._pathname;
  }
  set pathname(val) {
    const s = String(val);
    this._pathname = s.startsWith("/") ? s : "/" + s;
  }

  get search() {
    return this._search;
  }
  set search(val) {
    const s = String(val);
    this._search = !s || s === "?" ? "" : s.startsWith("?") ? s : "?" + s;
    this._searchParams = new URLSearchParamsPolyfill(this._search);
    this._searchParams._owner = this;
  }

  get searchParams() {
    return this._searchParams;
  }

  get hash() {
    return this._hash;
  }
  set hash(val) {
    const s = String(val);
    this._hash = !s || s === "#" ? "" : s.startsWith("#") ? s : "#" + s;
  }

  get origin() {
    if (this._scheme === "http" || this._scheme === "https" || this._scheme === "ftp" || this._scheme === "ws" || this._scheme === "wss") {
      return `${this._scheme}://${this.host}`;
    }
    return "null";
  }

  get href() {
    const auth =
      this.username || this.password
        ? `${encodeURIComponent(this.username)}${
            this.password ? ":" + encodeURIComponent(this.password) : ""
          }@`
        : "";
    const sep = this._hasSlashes || this._hostname ? "//" : "";
    return `${this._scheme}:${sep}${auth}${this.host}${this._pathname}${this._search}${this._hash}`;
  }
  set href(val) {
    const next = new URLPolyfill(val);
    this._scheme = next._scheme;
    this._hasSlashes = next._hasSlashes;
    this.username = next.username;
    this.password = next.password;
    this._hostname = next._hostname;
    this._port = next._port;
    this._pathname = next._pathname;
    this._search = next._search;
    this._hash = next._hash;
    this._searchParams = new URLSearchParamsPolyfill(this._search);
    this._searchParams._owner = this;
  }

  toString() {
    return this.href;
  }
  toJSON() {
    return this.href;
  }
}

class FinickyURL extends URLPolyfill {
  constructor(url, opener = null) {
    super(url);
    this._opener = opener;
  }

  get urlString() {
    console.warn(
      'Accessing legacy property "urlString" on URL object. Please use standard URL properties (e.g., url.href) instead.'
    );
    return this.href;
  }

  get url() {
    console.warn(
      'Accessing legacy property "url" on URL object. Please use standard URL properties instead.'
    );
    return {
      username: this.username,
      host: this.hostname,
      protocol: this.protocol.replace(":", ""),
      pathname: this.pathname,
      search: this.search.replace(/^\?/, ""),
      password: this.password,
      port: this.port ? parseInt(this.port, 10) : undefined,
      hash: this.hash.replace(/^#/, ""),
    };
  }

  get opener() {
    console.warn(
      'Accessing legacy property "opener" on URL object. Please use the second argument (options.opener) instead.'
    );
    return this._opener;
  }

  get keys() {
    throw new Error(
      'Accessing legacy property "keys" that is no longer supported, please use finicky.getModifierKeys() instead.'
    );
  }
}

globalThis.URL = URLPolyfill;
globalThis.URLSearchParams = URLSearchParamsPolyfill;
globalThis.FinickyURL = FinickyURL;

function matchHostnames(matchers) {
  const list = Array.isArray(matchers) ? matchers : [matchers];
  for (const m of list) {
    if (!(m instanceof RegExp) && typeof m !== "string") {
      throw new Error(`Unrecognized hostname type "${typeof m}"`);
    }
  }
  const fn = function (url) {
    const hostname = url.hostname;
    if (!hostname) {
      console.warn("No hostname available for", url.href);
      return false;
    }
    return list.some((m) => {
      if (m instanceof RegExp) return m.test(hostname);
      if (typeof m === "string") return m === hostname;
      return false;
    });
  };
  fn.__browsifyHostnames = list.map((m) =>
    m instanceof RegExp
      ? { kind: "regex", pattern: m.source, flags: m.flags || "" }
      : { kind: "exact", pattern: m }
  );
  return fn;
}

function matchDomains(matchers) {
  console.warn("finicky.matchDomains is deprecated. Use finicky.matchHostnames instead.");
  return matchHostnames(matchers);
}

function matchOpener(bundleIds) {
  const list = (Array.isArray(bundleIds) ? bundleIds : [bundleIds]).map(String);
  const fn = function (_url, options) {
    const bid = options?.opener?.bundleId;
    return Boolean(bid && list.includes(bid));
  };
  fn.__browsifyOpenerBundleIds = list;
  return fn;
}

function stripParams(patterns) {
  const list = (Array.isArray(patterns) ? patterns : [patterns]).map(String);
  return {
    __browsifyStripParams: list,
    match: "*",
    url: (url) => {
      for (const pat of list) {
        if (pat.endsWith("*")) {
          const prefix = pat.slice(0, -1);
          const keys = Array.from(url.searchParams.keys());
          for (const k of keys) {
            if (k.startsWith(prefix)) url.searchParams.delete(k);
          }
        } else {
          url.searchParams.delete(pat);
        }
      }
      return url;
    },
  };
}

function forceHttps(match = "*") {
  return {
    __browsifyForceHttps: true,
    match,
    url: (url) => {
      if (url.protocol === "http:") url.protocol = "https:";
      return url;
    },
  };
}

function replaceHost(fromOrMatch, toHost) {
  return {
    __browsifyReplaceHost: String(toHost),
    match: fromOrMatch,
    url: (url) => {
      url.host = String(toHost);
      return url;
    },
  };
}

globalThis.finicky = {
  matchHostnames,
  matchDomains,
  matchOpener,
  stripParams,
  forceHttps,
  replaceHost,
  getModifierKeys: () => {
    if (typeof globalThis.__browsifyHostGetModifierKeys === "function") {
      return JSON.parse(globalThis.__browsifyHostGetModifierKeys());
    }
    return {
      shift: false,
      option: false,
      command: false,
      control: false,
      capsLock: false,
      fn: false,
      function: false,
    };
  },
  getSystemInfo: () => {
    if (typeof globalThis.__browsifyHostGetSystemInfo === "function") {
      return JSON.parse(globalThis.__browsifyHostGetSystemInfo());
    }
    return { localizedName: "Mac", name: "mac" };
  },
  getPowerInfo: () => {
    if (typeof globalThis.__browsifyHostGetPowerInfo === "function") {
      return JSON.parse(globalThis.__browsifyHostGetPowerInfo());
    }
    return { isCharging: false, isConnected: true, percentage: 100 };
  },
  getBattery: () => {
    console.error("finicky.getBattery is unavailable. Use finicky.getPowerInfo instead. Returning dummy values.");
    return { isCharging: false, isPluggedIn: false, chargePercentage: 0 };
  },
  isAppRunning: (id) => {
    if (typeof globalThis.__browsifyHostIsAppRunning === "function") {
      return Boolean(globalThis.__browsifyHostIsAppRunning(String(id)));
    }
    return false;
  },
  notify: (_title, _subtitle) => {
    console.error("finicky.notify is unavailable. Use console.log, console.warn or console.error instead.");
    return undefined;
  },
};

function autodetectAppStringType(app) {
  if (app === null) return "none";
  if (/^[a-zA-Z0-9 ]+$/.test(app)) return "appName";
  if (/^[a-zA-Z0-9.-]+$/.test(app)) return "bundleId";
  if (/^(~?(?:\/[^/\n]+)+\/[^/\n]+\.app)$/.test(app)) return "path";
  return "appName";
}

function isLegacyURLObject(obj) {
  if (!obj || typeof obj !== "object" || obj instanceof URLPolyfill) return false;
  return (
    (typeof obj.username === "string" || obj.username === undefined) &&
    typeof obj.host === "string" &&
    (typeof obj.protocol === "string" || obj.protocol === undefined) &&
    (typeof obj.pathname === "string" || obj.pathname === undefined) &&
    (typeof obj.search === "string" || obj.search === undefined) &&
    (typeof obj.password === "string" || obj.password === undefined) &&
    (typeof obj.port === "number" || obj.port === undefined) &&
    (typeof obj.hash === "string" || obj.hash === undefined)
  );
}

function legacyURLObjectToString(urlObj) {
  const proto = urlObj.protocol ? urlObj.protocol.replace(":", "") : "https";
  const user = urlObj.username || "";
  const pass = urlObj.password ? ":" + urlObj.password : "";
  const auth = user || urlObj.password ? `${user}${pass}@` : "";
  const port = urlObj.port ? ":" + urlObj.port : "";
  let pathname = urlObj.pathname || "";
  if (pathname && !pathname.startsWith("/")) pathname = "/" + pathname;
  const search = urlObj.search ? "?" + String(urlObj.search).replace(/^\?/, "") : "";
  const hash = urlObj.hash ? "#" + String(urlObj.hash).replace(/^#/, "") : "";
  return `${proto}://${auth}${urlObj.host}${port}${pathname}${search}${hash}`;
}

function matchWildcardJS(pattern, str) {
  if (!pattern.includes("*")) {
    return pattern === str || (str.endsWith("/") && !pattern.endsWith("/") && pattern === str.slice(0, -1));
  }
  try {
    const PLACEHOLDER = "\u0000";
    const withEscaped = pattern.replace(/\\\*/g, PLACEHOLDER);
    let escaped = withEscaped.replace(/[.+?^${}()|[\]\\]/g, "\\$&");
    if (!/^\w+:/.test(pattern)) {
      if (!pattern.startsWith("*")) {
        escaped = `(?:https?:|ftp:|mailto:|file:|tel:|sms:|data:)?(?:\\/\\/)?${escaped}`;
      }
    } else {
      escaped = escaped.replace(/\//g, "\\/");
      if (escaped.endsWith("\\/\\/")) escaped += ".*";
    }
    const regexPat = escaped.replace(/\*/g, ".*?").replace(new RegExp(PLACEHOLDER, "g"), "\\*");
    const re = new RegExp(`^${regexPat}$`);
    if (re.test(str)) return true;
    if (str.endsWith("/") && !pattern.endsWith("/") && !pattern.endsWith("*")) {
      return re.test(str.slice(0, -1));
    }
    return false;
  } catch (_e) {
    return false;
  }
}

function isMatchJS(match, url, options) {
  if (Array.isArray(match)) {
    return match.some((m) => isMatchJS(m, url, options));
  }
  if (typeof match === "string") {
    if (match === "") return false;
    return matchWildcardJS(match, url.href);
  }
  if (match instanceof RegExp) {
    return match.test(url.href);
  }
  if (typeof match === "function") {
    return Boolean(match(url, options));
  }
  return false;
}

function rewriteUrlJS(rewrite, url, options) {
  if (rewrite instanceof FinickyURL) return rewrite;
  if (typeof rewrite === "string") return new FinickyURL(rewrite, options.opener || null);
  if (typeof rewrite === "function") {
    const res = rewrite(url, options);
    return rewriteUrlJS(res, url, options);
  }
  if (rewrite instanceof URLPolyfill) {
    return new FinickyURL(rewrite.href, options.opener || null);
  }
  if (isLegacyURLObject(rewrite)) {
    return new FinickyURL(legacyURLObjectToString(rewrite), options.opener || null);
  }
  return url;
}

function resolveBrowserSpecJS(spec, url, options) {
  const resolved = typeof spec === "function" ? spec(url, options) : spec;
  if (resolved === undefined) {
    throw new Error("Browser config cannot be undefined");
  }
  if (resolved === null) {
    return {
      name: "",
      appType: "none",
      profile: "",
      args: [],
      url: url.href,
    };
  }
  if (typeof resolved === "string") {
    const [name, rawProfile] = resolved.split(":");
    let profile = rawProfile || "";
    let container = undefined;
    if (profile.startsWith("container:")) {
      container = profile.slice("container:".length) || undefined;
      profile = "";
    }
    return {
      name,
      appType: autodetectAppStringType(name),
      profile,
      container,
      args: [],
      url: url.href,
    };
  }
  if (typeof resolved === "object" && typeof resolved.name === "string") {
    let profile = resolved.profile || "";
    let container = resolved.container || undefined;
    if (!container && profile.startsWith("container:")) {
      container = profile.slice("container:".length) || undefined;
      profile = "";
    }
    return {
      name: resolved.name,
      appType: resolved.appType || autodetectAppStringType(resolved.name),
      openInBackground:
        typeof resolved.openInBackground === "boolean" ? resolved.openInBackground : undefined,
      profile,
      container,
      appId: resolved.appId || undefined,
      appMode: typeof resolved.appMode === "boolean" ? resolved.appMode : undefined,
      args: Array.isArray(resolved.args) ? resolved.args.map(String) : [],
      url: url.href,
    };
  }
  throw new Error("Invalid browser specification");
}

globalThis.__browsifyInspectConfig = function () {
  let cfg = globalThis.__browsifyDefaultExport;
  if (!cfg && globalThis.module && globalThis.module.exports) {
    cfg = globalThis.module.exports;
  }
  if (cfg && cfg.default && typeof cfg.default === "object") {
    cfg = cfg.default;
  }
  if (!cfg || typeof cfg !== "object") {
    throw new Error("Invalid config: Configuration must export an object");
  }

  if (
    cfg.defaultBrowser !== undefined &&
    cfg.defaultBrowser !== null &&
    typeof cfg.defaultBrowser !== "string" &&
    typeof cfg.defaultBrowser !== "function" &&
    !(typeof cfg.defaultBrowser === "object" && typeof cfg.defaultBrowser.name === "string")
  ) {
    throw new Error("Invalid config: defaultBrowser must be a string, object, function, or null");
  }

  globalThis.__browsifyActiveConfig = cfg;

  // Safe marker-based lowering only: never regex-sniff arbitrary JS function bodies!
  function inspectMatcher(m) {
    if (typeof m === "string") {
      if (m === "*") {
        return { static: true, kind: "always" };
      }
      return { static: true, kind: "wildcard", pattern: m };
    }
    if (m instanceof RegExp) {
      return { static: true, kind: "regex", pattern: m.source, flags: m.flags || "" };
    }
    if (typeof m === "function" && Array.isArray(m.__browsifyHostnames)) {
      return { static: true, kind: "hostnames", patterns: m.__browsifyHostnames };
    }
    if (typeof m === "function" && Array.isArray(m.__browsifyOpenerBundleIds)) {
      return { static: true, kind: "openerBundleId", bundleIds: m.__browsifyOpenerBundleIds };
    }
    if (Array.isArray(m)) {
      const items = m.map(inspectMatcher);
      if (items.every((i) => i.static)) {
        return { static: true, kind: "any", items };
      }
      return { static: false };
    }
    return { static: false };
  }

  function inspectRewriteTransform(rw) {
    if (rw && Array.isArray(rw.__browsifyStripParams)) {
      return { static: true, kind: "stripParams", params: rw.__browsifyStripParams };
    }
    if (rw && rw.__browsifyForceHttps === true) {
      return { static: true, kind: "forceHttps" };
    }
    if (rw && typeof rw.__browsifyReplaceHost === "string") {
      return { static: true, kind: "replaceHost", host: rw.__browsifyReplaceHost };
    }
    if (rw && typeof rw.url === "string") {
      return { static: true, kind: "replaceUrl", url: rw.url };
    }
    return { static: false };
  }

  function inspectBrowser(b) {
    if (b === undefined) {
      return {
        static: true,
        explicit: false,
        spec: resolveBrowserSpecJS("com.apple.Safari", new URLPolyfill("https://example.com"), { opener: null }),
      };
    }
    if (b === null || typeof b === "string" || (typeof b === "object" && typeof b.name === "string")) {
      const spec = resolveBrowserSpecJS(b, new URLPolyfill("https://example.com"), { opener: null });
      delete spec.url;
      return { static: true, explicit: true, spec };
    }
    return { static: false, explicit: true };
  }

  const defBrowser = inspectBrowser(cfg.defaultBrowser);
  const rewrites = [];
  let allStatic = defBrowser.static;

  if (Array.isArray(cfg.rewrite)) {
    for (const r of cfg.rewrite) {
      const m = inspectMatcher(r.match);
      const t = inspectRewriteTransform(r);
      if (m.static && t.static) {
        rewrites.push({
          static: true,
          matcher: m,
          transform: t,
        });
      } else {
        allStatic = false;
        rewrites.push({ static: false });
      }
    }
  }

  const handlers = [];
  if (Array.isArray(cfg.handlers)) {
    for (const h of cfg.handlers) {
      const m = inspectMatcher(h.match);
      const b = inspectBrowser(h.browser);
      if (m.static && b.static) {
        handlers.push({ static: true, matcher: m, browser: b.spec });
      } else {
        allStatic = false;
        handlers.push({ static: false });
      }
    }
  }

  const opts = cfg.options || {};
  return JSON.stringify({
    allStatic,
    defaultBrowser: defBrowser,
    rewrites,
    handlers,
    options: {
      keepRunning: typeof opts.keepRunning === "boolean" ? opts.keepRunning : undefined,
      hideIcon: typeof opts.hideIcon === "boolean" ? opts.hideIcon : undefined,
      logRequests: typeof opts.logRequests === "boolean" ? opts.logRequests : undefined,
      checkForUpdates: typeof opts.checkForUpdates === "boolean" ? opts.checkForUpdates : undefined,
    },
  });
};

globalThis.__browsifyEvalRewriteStep = function (index, currentUrlStr, originalUrlStr, openerJson) {
  const cfg = globalThis.__browsifyActiveConfig;
  const rw = cfg?.rewrite?.[index];
  if (!rw) return currentUrlStr;

  const opener = openerJson ? JSON.parse(openerJson) : null;
  const url = new FinickyURL(currentUrlStr, opener);
  const options = {
    opener,
    originalUrl: new FinickyURL(originalUrlStr || currentUrlStr, opener),
  };

  if (isMatchJS(rw.match, url, options)) {
    const rewritten = rewriteUrlJS(rw.url, url, options);
    return rewritten.href;
  }
  return url.href;
};

globalThis.__browsifyEvalHandlerStep = function (index, currentUrlStr, originalUrlStr, openerJson) {
  const cfg = globalThis.__browsifyActiveConfig;
  const h = cfg?.handlers?.[index];
  if (!h) return "";

  const opener = openerJson ? JSON.parse(openerJson) : null;
  const url = new FinickyURL(currentUrlStr, opener);
  const options = {
    opener,
    originalUrl: new FinickyURL(originalUrlStr || currentUrlStr, opener),
  };

  if (isMatchJS(h.match, url, options)) {
    return JSON.stringify(resolveBrowserSpecJS(h.browser, url, options));
  }
  return "";
};

globalThis.__browsifyEvalDefaultBrowser = function (currentUrlStr, originalUrlStr, openerJson) {
  const cfg = globalThis.__browsifyActiveConfig;
  const opener = openerJson ? JSON.parse(openerJson) : null;
  const url = new FinickyURL(currentUrlStr, opener);
  const options = {
    opener,
    originalUrl: new FinickyURL(originalUrlStr || currentUrlStr, opener),
  };
  return JSON.stringify(resolveBrowserSpecJS(cfg.defaultBrowser, url, options));
};
})();
"##;
