(function () {
  "use strict";
  const s = document.currentScript as HTMLScriptElement | null;
  if (!s) return;
  const k = s.getAttribute("data-key");
  const a = s.getAttribute("data-api") || "";
  const e = a + "/api/collect";
  if (navigator.doNotTrack === "1") return;

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

  function pv() {
    send("pageview", {
      path: location.pathname + location.search,
      title: document.title,
      referrer: document.referrer,
      screen: screen.width + "x" + screen.height,
      language: navigator.language,
    });
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

  (window as any).pulse = function (action: string, ...args: any[]) {
    if (action === "event" && args[0]) {
      send("event", { name: args[0], data: args[1] || {}, path: location.pathname });
    } else if (action === "identify" && args[0]) {
      send("identify", { traits: args[0] });
    }
  };
})();
