;(function () {
  var settings = {}
  try {
    settings = JSON.parse(localStorage.getItem("sss.settings") || "{}") || {}
  } catch (e) {
    settings = {}
  }
  if (settings.theme === "light" || settings.theme === "dark") document.documentElement.dataset.theme = settings.theme
  var nav = navigator
  if (nav.globalPrivacyControl === true || nav.doNotTrack === "1" || settings.stats === false) return
  var ref = ""
  try {
    if (document.referrer) {
      var host = new URL(document.referrer).hostname
      if (host !== location.hostname) ref = host
    }
  } catch (e) {
    ref = ""
  }
  function send(e) {
    try {
      fetch("/api/hit", {
        method: "POST",
        body: JSON.stringify({ e: e, p: location.pathname, r: e === "view" ? ref : "", d: "", l: document.documentElement.lang }),
        headers: { "content-type": "application/json" },
        keepalive: true,
        credentials: "omit",
        cache: "no-store",
      }).catch(function () {})
    } catch (x) {}
  }
  send("view")
  document.addEventListener("click", function (ev) {
    var a = ev.target && ev.target.closest ? ev.target.closest("a[href*='/releases']") : null
    if (a) send("desktop")
  })
})()
