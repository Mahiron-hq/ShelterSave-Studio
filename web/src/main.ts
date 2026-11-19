import "./app.css"
import { mount } from "svelte"
import App from "./App.svelte"

const target = document.getElementById("app")
if (target) {
  target.replaceChildren()
  mount(App, { target })
}

if (import.meta.env.PROD && "serviceWorker" in navigator) {
  void navigator.serviceWorker.register("/sw.js").catch(() => {})
}
