<script lang="ts">
  import { Tabs, TabsList, TabsPanel, TabsTab } from "@ui-sdk/svelte/tabs"
  import Icon from "../components/Icon.svelte"
  import Resources from "./vault/Resources.svelte"
  import Storage from "./vault/Storage.svelte"
  import Rooms from "./vault/Rooms.svelte"
  import Themes from "./vault/Themes.svelte"
  import Book from "./vault/Book.svelte"
  import { app } from "../lib/state.svelte"

  const tabs = ["resources", "storage", "rooms", "themes", "book"]
</script>

<h1 class="section-title"><Icon name="house" /> {app.t("nav.vault")}</h1>
<Tabs value={app.vaultTab} onValueChange={(v) => v && (app.vaultTab = v)}>
  <TabsList aria-label={app.t("nav.vault")}>
    {#each tabs as t (t)}
      <TabsTab value={t}>{app.t(`vault.tab.${t}`)}</TabsTab>
    {/each}
  </TabsList>
  <TabsPanel value="resources">{#if app.vaultTab === "resources"}<Resources />{/if}</TabsPanel>
  <TabsPanel value="storage">{#if app.vaultTab === "storage"}<Storage />{/if}</TabsPanel>
  <TabsPanel value="rooms">{#if app.vaultTab === "rooms"}<Rooms />{/if}</TabsPanel>
  <TabsPanel value="themes">{#if app.vaultTab === "themes"}<Themes />{/if}</TabsPanel>
  <TabsPanel value="book">{#if app.vaultTab === "book"}<Book />{/if}</TabsPanel>
</Tabs>
