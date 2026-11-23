<script lang="ts">
  import Icon from "../components/Icon.svelte"
  import Check from "../components/Check.svelte"
  import BoolField from "../fields/BoolField.svelte"
  import NumField from "../fields/NumField.svelte"
  import NumPick from "../fields/NumPick.svelte"
  import PercentRow from "../fields/PercentRow.svelte"
  import DurationRow from "../fields/DurationRow.svelte"
  import TaskTimer from "../fields/TaskTimer.svelte"
  import Block from "./Block.svelte"
  import Incidents from "./Incidents.svelte"
  import { app } from "../lib/state.svelte"

  const r = $derived(app.root ?? {})
  const ms = $derived(r.MysteriousStranger)
  const dc = $derived(r.DeathclawManager)
  const bc = $derived(r.BottleAndCappyMgrSerializeKey)
  const rf = $derived(r.refugeeSpawner)
  const rm = $derived(r.ratingMgr)
  const dayRatings = $derived<number[]>((rm?.ratings ?? []).map((d: any) => d?.rating).filter((v: unknown) => typeof v === "number"))
  const cq = $derived(r.completedQuestDataManager)
  const shop = $derived(r.ShopWindow && typeof r.ShopWindow === "object" ? r.ShopWindow : null)
  const task = (v: unknown) => (typeof v === "number" && v >= 0 ? v : null)
</script>

<section class="panel">
  <h3>{app.t("o.events")}</h3>
  <p class="muted">{app.t("o.eventsHelp")}</p>
</section>

{#if ms}
  <section class="panel">
    <h3>{app.t("o.stranger")}</h3>
    <p class="muted">{app.t("o.strangerHelp")}</p>
    {#if ms.canAppear !== undefined}<BoolField path={["MysteriousStranger", "canAppear"]} label={app.t("o.strangerCan")} help={app.t("o.strangerCanHelp")} />{/if}
    {#if ms.currentState === "Hiding"}
      {#if typeof ms.timeToAppear === "number"}
        <DurationRow
          path={["MysteriousStranger", "remainingTimeToAppear"]}
          label={app.t("o.strangerIn")}
          help={app.t("o.strangerInHelp")}
          secs={Math.max(0, ms.timeToAppear - (ms.remainingTimeToAppear ?? 0))}
          unit={1}
          onset={(s) => app.act({ action: "setStrangerTime", seconds: s })}
        />
        <button type="button" title={app.t("o.strangerNowHelp")} onclick={() => app.act({ action: "setStrangerTime", seconds: 10 })}><Icon name="sparkles" /> {app.t("o.strangerNow")}</button>
      {/if}
    {:else}
      <p><span class="muted">{app.t("o.status")}</span> {app.t("o.strangerHere")}</p>
      {#if typeof ms.timeToHide === "number"}
        <DurationRow
          path={["MysteriousStranger", "remainingTimeToHide"]}
          label={app.t("o.strangerLeaves")}
          help={app.t("o.strangerLeavesHelp")}
          secs={Math.max(0, ms.timeToHide - (ms.remainingTimeToHide ?? 0))}
          unit={1}
          onset={(s) => app.act({ action: "setStrangerTime", seconds: s })}
        />
      {/if}
    {/if}
  </section>
{/if}

{#if dc}
  <section class="panel">
    <h3>{app.t("o.deathclaw")}</h3>
    <p class="muted">{app.t("o.deathclawHelp")}</p>
    {#if dc.canDeathclawEmergencyOccurs !== undefined}<BoolField path={["DeathclawManager", "canDeathclawEmergencyOccurs"]} label={app.t("o.deathclawCan")} help={app.t("o.deathclawCanHelp")} />{/if}
    <PercentRow path={["DeathclawManager", "deathclawTotalExtraChance"]} label={app.t("o.deathclawChance")} help={app.t("o.deathclawChanceHelp")} />
    {#if task(dc.deathclawCooldownID) !== null}<TaskTimer id={dc.deathclawCooldownID} label={app.t("o.deathclawCooldown")} help={app.t("o.deathclawCooldownHelp")} />{/if}
  </section>
{/if}

{#if r.vault?.emergencyData}
  <section class="panel"><Incidents /></section>
{/if}

{#if bc}
  <section class="panel">
    <h3>{app.t("o.bottle")}</h3>
    <p class="muted">{app.t("o.bottleHelp")}</p>
    {#if bc.SerializeLocked !== undefined}<BoolField path={["BottleAndCappyMgrSerializeKey", "SerializeLocked"]} label={app.t("o.bottleLocked")} help={app.t("o.bottleLockedHelp")} />{/if}
    <PercentRow path={["BottleAndCappyMgrSerializeKey", "SerializeAccumulatedTriggerChance"]} label={app.t("o.bottleChance")} help={app.t("o.bottleChanceHelp")} />
    {#if task(bc.SerializeUnlockTask) !== null}<TaskTimer id={bc.SerializeUnlockTask} label={app.t("o.bottleUnlockIn")} help={app.t("o.bottleUnlockInHelp")} />{/if}
  </section>
{/if}

{#if rf}
  <section class="panel">
    <h3>{app.t("o.refugees")}</h3>
    <p class="muted">{app.t("o.refugeesHelp")}</p>
    {#if typeof rf.currentPool === "string" && rf.currentPool}
      <p><span class="muted">{app.t("o.refugeesPool")}</span> {app.t("o.refugeesPoolN", { n: rf.currentPool.replace(/^Pool/, "") })}</p>
    {/if}
    {#if rf.collectedAllRefugees !== undefined}<BoolField path={["refugeeSpawner", "collectedAllRefugees"]} label={app.t("o.refugeesAll")} help={app.t("o.refugeesAllHelp")} />{/if}
    {#each [["retrievedDwellers", "o.refugeesGot", "o.refugeesGotHelp"], ["failedChances", "o.refugeesFailed", "o.refugeesFailedHelp"]] as [k, label, help] (k)}
      {#if rf.currentPoolData?.[k] !== undefined}<NumField path={["refugeeSpawner", "currentPoolData", k]} label={app.t(label)} help={app.t(help)} min={0} max={20} />{/if}
    {/each}
    {#each ["globalTimerTask", "newGameTask", "succeed"] as k (k)}
      {#if task(rf[k]) !== null}<TaskTimer id={rf[k]} label={app.t("o.refugeesIn")} help={app.t("o.refugeesInHelp")} />{/if}
    {/each}
  </section>
{/if}

{#if rm}
  <section class="panel">
    <h3>{app.t("o.rating")}</h3>
    <p class="muted">{app.t("o.ratingHelp")}</p>
    {#if !(rm.ratings ?? []).length}<p class="muted">{app.t("o.ratingNone")}</p>{/if}
    <div class="days">
      {#each rm.ratings ?? [] as day, k (k)}
        <p>
          <span>{app.t("o.ratingDay", { n: k + 1 })}</span>
          <span class="pick">
            {#if typeof day?.rating === "number"}
              <NumPick value={day.rating} label={app.t("o.ratingDay", { n: k + 1 })} lo={0} hi={100} slider={[0, 100]} oncommit={(v) => app.act({ action: "setDayRating", index: k, rating: v })} />
              <span class="muted">%</span>
            {/if}
          </span>
          <span>
            {#if typeof day?.collected === "boolean"}
              <span title={app.t("o.ratingCollectedHelp")}>
                <Check checked={day.collected} label={app.t("o.ratingCollected")} onchange={(v) => app.act({ action: "setDayRating", index: k, collected: v })} />
              </span>
            {/if}
          </span>
        </p>
      {/each}
    </div>
    <div class="days totals">
      {#if dayRatings.length > 0 && dayRatings.length < 7}
        <p title={app.t("o.ratingNowHelp")}><span class="muted">{app.t("o.ratingNow")}</span> <span>{Math.ceil(dayRatings.reduce((a, b) => a + b, 0) / dayRatings.length)}%</span></p>
      {/if}
      {#each [["weekRating", "o.ratingWeek", "o.ratingWeekHelp"], ["lastWeekRating", "o.ratingLastWeek", ""]] as [k, label, help] (k)}
        {#if typeof rm[k] === "number"}<p title={help ? app.t(help) : undefined}><span class="muted">{app.t(label)}</span> <span>{rm[k]}%</span></p>{/if}
      {/each}
    </div>
  </section>
{/if}

{#if cq}
  <section class="panel">
    <h3>{app.t("o.questTimers")}</h3>
    {#if task(cq.taskID) !== null}<TaskTimer id={cq.taskID} label={app.t("o.newQuestsIn")} help={app.t("o.newQuestsInHelp")} />{/if}
    {#if cq.nukaQuantumIncrement !== undefined}
      <NumField path={["completedQuestDataManager", "nukaQuantumIncrement"]} label={app.t("o.questQuantum")} help={app.t("o.questQuantumHelp")} min={0} max={10} />
    {/if}
  </section>
{/if}

{#if shop && Object.keys(shop).length}
  <section class="panel">
    <h3>{app.t("o.shop")}</h3>
    {#each Object.entries(shop) as [k, v] (k)}
      {#if typeof v === "boolean"}<BoolField path={["ShopWindow", k]} />{/if}
    {/each}
  </section>
{/if}

{#if app.expert}
  <Block
    titleKey="o.eventsRaw"
    roots={["MysteriousStranger", "DeathclawManager", "BottleAndCappyMgrSerializeKey", "refugeeSpawner", "ShopWindow", "seasonEventData", "dayToDayRewardMgr", "PromotionFlags", "ratingMgr", "LunchBoxCollectWindow", "SeasonShopWarningModal", "PromoCodesWindow", "JunkGiveAwayWindow", "swrveEventsManager"]}
  />
{/if}

<style>
  p {
    margin: 4px 0;
  }
  .days {
    display: grid;
    grid-template-columns: max-content max-content 1fr;
    column-gap: var(--s-md);
  }
  .days p {
    display: grid;
    grid-column: 1 / -1;
    grid-template-columns: subgrid;
    align-items: center;
    min-height: 32px;
    margin: 0;
  }
  .totals {
    grid-template-columns: max-content 1fr;
    margin-top: var(--s-sm);
  }
  .totals p {
    min-height: 24px;
  }
  .totals p > span:first-child {
    min-width: 0;
    padding-inline-start: 0;
  }
  .pick {
    display: flex;
    gap: 6px;
    align-items: center;
  }
</style>
