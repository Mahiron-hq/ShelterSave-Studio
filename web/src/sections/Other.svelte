<script lang="ts">
  import { Tabs, TabsList, TabsPanel, TabsTab } from "@ui-sdk/svelte/tabs"
  import Icon from "../components/Icon.svelte"
  import Quests from "../other/Quests.svelte"
  import Wasteland from "../other/Wasteland.svelte"
  import Objectives from "../other/Objectives.svelte"
  import Stats from "../other/Stats.svelte"
  import Tutorial from "../other/Tutorial.svelte"
  import Events from "../other/Events.svelte"
  import Block from "../other/Block.svelte"
  import JsonTree from "../other/JsonTree.svelte"
  import { app } from "../lib/state.svelte"

  const SERVICE_KNOWN = [
    "timeMgr", "localNotificationMgr", "taskMgr", "ratingMgr", "specialTheme", "dwellers", "constructMgr", "vault",
    "dwellerSpawner", "deviceName", "tutorialManager", "objectiveMgr", "unlockableMgr", "survivalW", "ShopWindow",
    "happinessManager", "refugeeSpawner", "LunchBoxCollectWindow", "SeasonShopWarningModal", "DeathclawManager",
    "PromoCodesWindow", "JunkGiveAwayWindow", "MysteriousStranger", "StatsWindow", "PromotionFlags", "appVersion",
    "BottleAndCappyMgrSerializeKey", "completedQuestDataManager", "seasonEventData", "dayToDayRewardMgr", "seasonId",
    "cameraPosition", "questSetup", "questDataManager", "questDwellers", "questDwellerSpawner", "task", "time",
    "viewManager", "versionCount", "swrveEventsManager",
  ]
  const SERVICE = [
    "taskMgr", "task", "time", "timeMgr", "cameraPosition", "viewManager", "localNotificationMgr", "happinessManager",
    "constructMgr", "survivalW", "questSetup", "questDataManager", "questDwellers", "questDwellerSpawner",
    "completedQuestDataManager", "objectiveMgr", "unlockableMgr", "tutorialManager", "StatsWindow", "DeathclawManager",
    "MysteriousStranger",
  ]
  const service = $derived([...SERVICE, ...Object.keys(app.root ?? {}).filter((k) => !SERVICE_KNOWN.includes(k))])
  const tabs = $derived(["quests", "wasteland", "objectives", "stats", "tutorial", "events", ...(app.expert ? ["service", "tree"] : [])])

  $effect(() => {
    if (!app.expert && (app.otherTab === "service" || app.otherTab === "tree")) app.otherTab = "quests"
  })
</script>

<h1 class="section-title"><Icon name="layers" /> {app.t("nav.other")}</h1>
<Tabs value={app.otherTab} onValueChange={(v) => v && (app.otherTab = v)}>
  <TabsList aria-label={app.t("nav.other")}>
    {#each tabs as t (t)}
      <TabsTab value={t}>{app.t(`o.${t}`)}</TabsTab>
    {/each}
  </TabsList>
  {#each tabs as t (t)}
    <TabsPanel value={t}>
      {#if app.otherTab === t}
        {#if t === "quests"}<Quests />
        {:else if t === "wasteland"}<Wasteland />
        {:else if t === "objectives"}<Objectives />
        {:else if t === "stats"}<Stats />
        {:else if t === "tutorial"}<Tutorial />
        {:else if t === "events"}<Events />
        {:else if t === "service"}
          <p class="muted">{app.t("o.serviceHelp")}</p>
          <Block titleKey="o.service" roots={service} />
        {:else if t === "tree"}<JsonTree />
        {/if}
      {/if}
    </TabsPanel>
  {/each}
</Tabs>
