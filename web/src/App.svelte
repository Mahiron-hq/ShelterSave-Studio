<script lang="ts">
  import { onMount } from "svelte"
  import { FileInput, FileInputDropzone } from "@ui-sdk/svelte/file-input"
  import Toolbar from "./components/Toolbar.svelte"
  import Nav from "./components/Nav.svelte"
  import Start from "./components/Start.svelte"
  import Toasts from "./components/Toasts.svelte"
  import Modals from "./dialogs/Modals.svelte"
  import Notice from "./components/Notice.svelte"
  import Icon from "./components/Icon.svelte"
  import Footer from "./components/Footer.svelte"
  import SaveSection from "./sections/Save.svelte"
  import VaultSection from "./sections/Vault.svelte"
  import DwellersSection from "./sections/Dwellers.svelte"
  import OtherSection from "./sections/Other.svelte"
  import { loadCore } from "./lib/core"
  import { app } from "./lib/state.svelte"
  import { ui } from "./lib/ui.svelte"
  import { MAX_FILE, openFile } from "./lib/files"
  import { readSession } from "./lib/session"
  import { hit } from "./lib/stats"

  let failed = $state(false)
  let pending: File | null = null

  onMount(async () => {
    try {
      await loadCore()
    } catch {
      failed = true
      return
    }
    app.init()
    const wanted = app.wantedLang()
    if (app.settings.lang !== "system" && wanted !== app.lang) {
      location.replace(`/${wanted}/`)
      return
    }
    app.ready = true
    hit("view")
    if (app.settings.keepSession) {
      const s = await readSession()
      if (s) app.push({ kind: "restoreSession", fileName: s.fileName, savedAt: s.savedAt, session: s.session })
    }
  })

  $effect(() => {
    void app.rev
    if (!app.isOpen) ui.reset()
  })

  function dropped(file: File) {
    if (app.dirty) {
      pending = file
      app.push({ kind: "custom", id: "replaceFile", data: () => pending && openFile(pending) })
    } else void openFile(file)
  }

  function keys(e: KeyboardEvent) {
    if (!app.ready || app.modals.length) return
    const t = e.target as HTMLElement | null
    const typing = !!t && (t.tagName === "INPUT" || t.tagName === "TEXTAREA" || t.isContentEditable)
    const mod = e.ctrlKey || e.metaKey
    if (mod && !e.altKey) {
      const k = e.key.toLowerCase()
      if (typing && (k === "z" || k === "y")) return
      if ((k === "z" && e.shiftKey) || k === "y") {
        e.preventDefault()
        app.redo()
      } else if (k === "z") {
        e.preventDefault()
        app.undo()
      } else if (k === "s" && app.isOpen) {
        e.preventDefault()
        app.push({ kind: "review" })
      }
    } else if (e.altKey && !mod && app.isOpen && ["1", "2", "3", "4"].includes(e.key)) {
      e.preventDefault()
      app.section = (["save", "vault", "dwellers", "other"] as const)[Number(e.key) - 1]
    }
  }
</script>

<svelte:window onkeydown={keys} />

<a class="skip" href="#main">{app.ready ? app.t("web.skip") : "Skip to content"}</a>

{#if failed}
  <main id="main" class="center">
    <div class="callout danger" role="alert">
      <Icon name="alert" />
      <p>{document.documentElement.lang === "ru" ? "Редактор не запустился в этом браузере. Обновите браузер или попробуйте другой." : "The editor could not start in this browser. Update the browser or try another one."}</p>
    </div>
  </main>
{:else if !app.ready}
  <main id="main" class="center" aria-busy="true">
    <p class="muted">{document.documentElement.lang === "ru" ? "Загрузка редактора…" : "Loading the editor…"}</p>
  </main>
{:else}
  <Toolbar />
  <div class="body" class:open={app.isOpen}>
    {#if app.isOpen}
      <Nav />
    {/if}
    <main id="main" tabindex="-1">
      {#if !app.isOpen}
        <Start />
      {:else if app.section === "save"}
        <SaveSection />
      {:else if app.section === "vault"}
        <VaultSection />
      {:else if app.section === "dwellers"}
        <DwellersSection />
      {:else}
        <OtherSection />
      {/if}
    </main>
  </div>
  {#if app.isOpen}
    <FileInput
      maxSize={MAX_FILE}
      value={[]}
      onValueChange={(files) => {
        if (files[0]) dropped(files[0])
      }}
    >
      <FileInputDropzone target="document" class="page-drop" aria-hidden="true">
        <p>{app.t("start.drop")}</p>
      </FileInputDropzone>
    </FileInput>
  {/if}
  <Footer />
  <Notice />
  <Modals />
  <Toasts />
{/if}

<style>
  .skip {
    position: absolute;
    inset-inline-start: 8px;
    inset-block-start: -40px;
    z-index: 100;
    background: var(--surface);
    padding: 6px 10px;
    border-radius: var(--r-md);
  }
  .skip:focus {
    inset-block-start: 8px;
  }
  .center {
    display: flex;
    min-height: 60vh;
    align-items: center;
    justify-content: center;
    padding: var(--s-lg);
  }
  .body {
    display: block;
  }
  .body.open {
    display: grid;
    grid-template-columns: 210px minmax(0, 1fr);
    min-height: calc(100vh - 50px);
  }
  main {
    padding: var(--s-lg);
    min-width: 0;
    outline: none;
  }
  :global([data-slot="file-input-dropzone"].page-drop) {
    display: none;
  }
  :global([data-slot="file-input-dropzone"].page-drop[data-dragging]) {
    display: flex;
    position: fixed;
    inset: 0;
    z-index: 90;
    align-items: center;
    justify-content: center;
    background: var(--accent-soft);
    border: 3px dashed var(--accent);
    font-size: 22px;
    font-weight: 600;
  }
  @media (max-width: 820px) {
    .body.open {
      display: block;
    }
    main {
      padding: var(--s-md);
    }
  }
</style>
