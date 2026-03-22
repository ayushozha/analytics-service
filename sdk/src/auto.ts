(function () {
  "use strict";
  const s = document.currentScript as HTMLScriptElement | null;
  if (!s) return;
  const k = s.getAttribute("data-key");
  const a = s.getAttribute("data-api") || "";
  const e = a + "/api/collect";

  // Feature flags from data attributes (all default on except heavy ones)
  const cfg = {
    respectDnt: s.getAttribute("data-dnt") !== "false",
    trackUtm: s.getAttribute("data-utm") !== "false",
    trackScrollDepth: s.getAttribute("data-scroll") === "true",
    trackWebVitals: s.getAttribute("data-vitals") === "true",
    trackOutlinks: s.getAttribute("data-outlinks") === "true",
    trackErrors: s.getAttribute("data-errors") === "true",
    trackClicks: s.getAttribute("data-clicks") === "true",
    trackSearch: s.getAttribute("data-search") === "true",
    searchParam: s.getAttribute("data-search-param") || "q",
  };

  if (cfg.respectDnt && navigator.doNotTrack === "1") return;

  function h(t: string): string {
    let c = 0;
    for (let i = 0; i < t.length; i++) {
      c = ((c << 5) - c) + t.charCodeAt(i);
      c |= 0;
    }
    return "v_" + Math.abs(c).toString(36);
  }

  const fp = [
    screen.width,
    screen.height,
    Intl.DateTimeFormat().resolvedOptions().timeZone,
    navigator.language,
    navigator.userAgent,
  ].join("|");

  let vid = h(fp);
  try {
    const sv = sessionStorage.getItem("_pv");
    if (sv) vid = sv;
    else sessionStorage.setItem("_pv", vid);
  } catch {}

  function send(type: string, payload: Record<string, unknown>) {
    const body = JSON.stringify({ type, payload, visitor_id: vid });
    if (navigator.sendBeacon) {
      navigator.sendBeacon(e + "?key=" + k, body);
    } else {
      fetch(e, {
        method: "POST",
        headers: { "Content-Type": "application/json", "X-Pulse-Key": k! },
        body,
        keepalive: true,
      });
    }
  }

  // --- UTM extraction ---
  function getUtmParams(): Record<string, string> {
    const params: Record<string, string> = {};
    if (!cfg.trackUtm) return params;
    try {
      const sp = new URLSearchParams(location.search);
      for (const key of ["utm_source", "utm_medium", "utm_campaign", "utm_content", "utm_term"]) {
        const val = sp.get(key);
        if (val) params[key] = val;
      }
      // Persist UTMs in sessionStorage so subsequent pageviews carry them
      if (Object.keys(params).length > 0) {
        sessionStorage.setItem("_putm", JSON.stringify(params));
      } else {
        const stored = sessionStorage.getItem("_putm");
        if (stored) Object.assign(params, JSON.parse(stored));
      }
    } catch {}
    return params;
  }

  // --- Pageview ---
  function pv() {
    const utm = getUtmParams();
    send("pageview", {
      path: location.pathname + location.search,
      title: document.title,
      referrer: document.referrer,
      screen: screen.width + "x" + screen.height,
      language: navigator.language,
      ...utm,
    });

    // Track site search if enabled and search param is present
    if (cfg.trackSearch) {
      try {
        const sp = new URLSearchParams(location.search);
        const query = sp.get(cfg.searchParam);
        if (query) {
          send("search_query", {
            query,
            path: location.pathname,
          });
        }
      } catch {}
    }
  }

  pv();

  const op = history.pushState;
  history.pushState = function (...args: Parameters<typeof history.pushState>) {
    op.apply(this, args);
    pv();
  };
  const or = history.replaceState;
  history.replaceState = function (...args: Parameters<typeof history.replaceState>) {
    or.apply(this, args);
    pv();
  };
  window.addEventListener("popstate", pv);

  // --- Scroll Depth Tracking ---
  if (cfg.trackScrollDepth) {
    let maxScroll = 0;
    let lastPath = location.pathname;
    const reportScroll = () => {
      if (maxScroll > 0) {
        send("scroll_depth", { path: lastPath, max_depth: maxScroll });
      }
    };
    const updateScroll = () => {
      const docHeight = Math.max(
        document.body.scrollHeight, document.documentElement.scrollHeight
      );
      const winHeight = window.innerHeight;
      const scrollTop = window.scrollY || document.documentElement.scrollTop;
      if (docHeight <= winHeight) { maxScroll = 100; return; }
      const pct = Math.round((scrollTop / (docHeight - winHeight)) * 100);
      if (pct > maxScroll) maxScroll = pct;
    };
    window.addEventListener("scroll", updateScroll, { passive: true });
    window.addEventListener("beforeunload", reportScroll);
    // Reset on SPA navigation
    const origPush2 = history.pushState;
    history.pushState = function (...args: Parameters<typeof history.pushState>) {
      reportScroll();
      maxScroll = 0;
      lastPath = location.pathname;
      origPush2.apply(this, args);
    };
  }

  // --- Core Web Vitals ---
  if (cfg.trackWebVitals) {
    const sendVital = (name: string, value: number, rating?: string) => {
      send("web_vital", { name, value: Math.round(value * 1000) / 1000, rating, path: location.pathname });
    };
    try {
      // Use PerformanceObserver for CLS, LCP, INP, FCP
      if (typeof PerformanceObserver !== "undefined") {
        // Largest Contentful Paint
        new PerformanceObserver((list) => {
          const entries = list.getEntries();
          const last = entries[entries.length - 1] as any;
          if (last) {
            const val = last.startTime;
            sendVital("LCP", val, val <= 2500 ? "good" : val <= 4000 ? "needs-improvement" : "poor");
          }
        }).observe({ type: "largest-contentful-paint", buffered: true });

        // First Contentful Paint
        new PerformanceObserver((list) => {
          for (const entry of list.getEntries()) {
            if (entry.name === "first-contentful-paint") {
              const val = entry.startTime;
              sendVital("FCP", val, val <= 1800 ? "good" : val <= 3000 ? "needs-improvement" : "poor");
            }
          }
        }).observe({ type: "paint", buffered: true });

        // Cumulative Layout Shift
        let clsValue = 0;
        new PerformanceObserver((list) => {
          for (const entry of list.getEntries()) {
            if (!(entry as any).hadRecentInput) {
              clsValue += (entry as any).value;
            }
          }
        }).observe({ type: "layout-shift", buffered: true });
        window.addEventListener("beforeunload", () => {
          sendVital("CLS", clsValue, clsValue <= 0.1 ? "good" : clsValue <= 0.25 ? "needs-improvement" : "poor");
        });

        // Interaction to Next Paint
        let inpValue = 0;
        new PerformanceObserver((list) => {
          for (const entry of list.getEntries()) {
            const dur = (entry as any).duration || 0;
            if (dur > inpValue) inpValue = dur;
          }
        }).observe({ type: "event", buffered: true });
        window.addEventListener("beforeunload", () => {
          if (inpValue > 0) {
            sendVital("INP", inpValue, inpValue <= 200 ? "good" : inpValue <= 500 ? "needs-improvement" : "poor");
          }
        });
      }

      // TTFB via Navigation Timing
      if (performance && performance.getEntriesByType) {
        const nav = performance.getEntriesByType("navigation")[0] as PerformanceNavigationTiming | undefined;
        if (nav) {
          const ttfb = nav.responseStart - nav.requestStart;
          sendVital("TTFB", ttfb, ttfb <= 800 ? "good" : ttfb <= 1800 ? "needs-improvement" : "poor");
        }
      }
    } catch {}
  }

  // --- Outlink & Download Tracking ---
  if (cfg.trackOutlinks) {
    document.addEventListener("click", (ev) => {
      const el = (ev.target as Element)?.closest("a") as HTMLAnchorElement | null;
      if (!el || !el.href) return;
      try {
        const url = new URL(el.href, location.href);
        // External link
        if (url.hostname !== location.hostname) {
          send("outlink", { url: el.href, link_type: "outlink", path: location.pathname });
          return;
        }
        // Download (common file extensions)
        const ext = url.pathname.split(".").pop()?.toLowerCase();
        const dlExts = ["pdf", "zip", "rar", "7z", "gz", "tar", "doc", "docx", "xls", "xlsx", "ppt", "pptx", "csv", "exe", "dmg", "apk", "ipa"];
        if (ext && dlExts.includes(ext)) {
          send("outlink", { url: el.href, link_type: "download", path: location.pathname });
        }
      } catch {}
    }, true);
  }

  // --- JS Error Tracking ---
  if (cfg.trackErrors) {
    window.addEventListener("error", (ev) => {
      send("js_error", {
        message: ev.message || "Unknown error",
        filename: ev.filename,
        lineno: ev.lineno,
        colno: ev.colno,
        stack: ev.error?.stack,
        path: location.pathname,
      });
    });
    window.addEventListener("unhandledrejection", (ev) => {
      const reason = ev.reason;
      send("js_error", {
        message: reason?.message || String(reason) || "Unhandled promise rejection",
        stack: reason?.stack,
        path: location.pathname,
      });
    });
  }

  // --- Click Heatmap Tracking ---
  if (cfg.trackClicks) {
    document.addEventListener("click", (ev) => {
      const target = ev.target as Element;
      let selector = "";
      try {
        selector = target.tagName?.toLowerCase() || "";
        if (target.id) selector += "#" + target.id;
        else if (target.className && typeof target.className === "string") {
          selector += "." + target.className.trim().split(/\s+/).slice(0, 2).join(".");
        }
      } catch {}
      send("click_event", {
        path: location.pathname,
        x: ev.clientX / window.innerWidth,
        y: (ev.clientY + window.scrollY) / Math.max(document.documentElement.scrollHeight, 1),
        element_selector: selector,
        viewport_width: window.innerWidth,
        viewport_height: window.innerHeight,
      });
    });
  }

  // --- Public API ---
  (window as any).pulse = function (action: string, ...args: any[]) {
    if (action === "event" && args[0]) {
      const eventPayload: Record<string, unknown> = {
        name: args[0],
        data: args[1] || {},
        path: location.pathname,
      };
      // Support revenue: pulse("event", "purchase", { item: "x" }, 99.99, "USD")
      if (typeof args[2] === "number") {
        eventPayload.revenue_amount = args[2];
        eventPayload.revenue_currency = args[3] || "USD";
      }
      send("event", eventPayload);
    } else if (action === "identify" && args[0]) {
      send("identify", { traits: args[0] });
    } else if (action === "search" && args[0]) {
      send("search_query", {
        query: args[0],
        results_count: args[1],
        path: location.pathname,
      });
    } else if (action === "survey_response" && args[0]) {
      send("survey_response", {
        survey_id: args[0],
        answers: args[1] || [],
        completed: args[2] !== false,
        path: location.pathname,
      });
    }
  };
})();
