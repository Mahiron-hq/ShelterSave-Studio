<script lang="ts">
  import ArtNotice from "../../art/ArtNotice.svelte"
  import Fold from "../../components/Fold.svelte"
  import Check from "../../components/Check.svelte"
  import NumPick from "../../fields/NumPick.svelte"
  import NumField from "../../fields/NumField.svelte"
  import ObjectForm from "../../fields/ObjectForm.svelte"
  import RoomPicture from "../../rooms/RoomPicture.svelte"
  import ThemePicker from "../../rooms/ThemePicker.svelte"
  import ThemeCard from "../../rooms/ThemeCard.svelte"
  import { catalog, consts, core, query, themePicture } from "../../lib/core"
  import { app } from "../../lib/state.svelte"

  const c = consts()
  const rooms = $derived(
    c.themeRooms
      .map((room) => ({ room, list: (catalog().themes as any[]).filter((t) => t.room === room) }))
      .filter((r) => r.list.length),
  )

  function inSeason(start: [number, number], end: [number, number]): boolean {
    const d = new Date()
    const today = [d.getMonth() + 1, d.getDate()]
    const le = (a: number[], b: number[]) => a[0] < b[0] || (a[0] === b[0] && a[1] <= b[1])
    return le(start, end) ? le(start, today) && le(today, end) : le(start, today) || le(today, end)
  }
  const pad = (n: number) => String(n).padStart(2, "0")
</script>

<section class="panel">
  <h3>{app.t("theme.title")}</h3>
  <p class="muted">{app.t("theme.help")}</p>
  <ArtNotice />
  {#if app.root?.specialTheme?.themeByRoomType === undefined}
    <p class="muted">{app.t("theme.noData")}</p>
  {:else}
    {#each rooms as { room, list } (room)}
      {@const cur = app.root?.specialTheme?.themeByRoomType?.[room] ?? null}
      {@const pic = themePicture(room, cur)}
      <div class="room">
        <h4>{room === "FakeWasteland" ? app.t("theme.exterior") : core().kindName(room, 1)}</h4>
        <ThemePicker {room} />
        <RoomPicture kind={pic[0]} id={pic[1]} maxW={420} noteMissing />
        <Fold title={app.t("theme.states")}>
          <table class="grid states">
            <tbody>
              {#each list as t (t.id)}
                {@const st = (void app.rev, query<[number, boolean]>("themeState", { id: t.id }))}
                <tr>
                  <td><ThemeCard id={t.id} /></td>
                  <td>
                    <NumPick
                      value={st[0]}
                      label={app.t("theme.partsOf")}
                      lo={0}
                      hi={c.themeParts}
                      oncommit={(v) => app.act({ action: "setThemeState", id: t.id, parts: v, crafted: st[1] && v >= c.themeParts })}
                    />
                    <span class="muted">{app.t("theme.partsOf")}</span>
                  </td>
                  <td title={app.t("theme.craftedHelp")}>
                    <Check
                      checked={st[1]}
                      label={app.t("theme.crafted")}
                      onchange={(v) => app.act({ action: "setThemeState", id: t.id, parts: v ? c.themeParts : st[0], crafted: v })}
                    />
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </Fold>
      </div>
    {/each}
  {/if}
</section>

<section class="panel">
  <h3>{app.t("theme.holidays")}</h3>
  <p class="muted">{app.t("theme.holidaysHelp")}</p>
  {#each c.holidays as [key, id] (key)}
    {@const ev = (catalog().eventThemes as any[]).find((e) => e.id === id)}
    {#if ev}
      <p class="row-wrap">
        <strong>{app.t(`theme.holiday.${key}`)}</strong>
        <span class="muted">{app.t("theme.dates", { from: `${pad(ev.start[1])}.${pad(ev.start[0])}`, to: `${pad(ev.end[1])}.${pad(ev.end[0])}` })}</span>
        {#if inSeason(ev.start, ev.end)}<span class="ok">{app.t("theme.now")}</span>{/if}
      </p>
    {/if}
  {/each}
</section>

{#if app.expert}
  <section class="panel">
    <h3>{app.t("theme.expert")}</h3>
    <NumField path={["vault", "VaultTheme"]} help={app.t("theme.vaultThemeHelp")} min={0} max={3} />
    <h4>{app.t("vault.roomThemes")}</h4>
    <ObjectForm path={["specialTheme", "themeByRoomType"]} />
    <h4>{app.t("vault.eventThemes")}</h4>
    <ObjectForm path={["specialTheme", "eventsThemes"]} />
  </section>
{/if}

<style>
  .room {
    margin-top: var(--s-lg);
  }
  h4 {
    margin: var(--s-sm) 0;
  }
  .states td {
    vertical-align: middle;
  }
  .ok {
    color: var(--success);
  }
</style>
