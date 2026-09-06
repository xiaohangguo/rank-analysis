<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { Effect, getCurrentWindow } from '@tauri-apps/api/window'

interface Player {
  championId: number
  isLoading: boolean
  summoner: { gameName: string; tagLine: string; puuid: string }
  rank?: {
    queueMap?: { RANKED_SOLO_5x5?: { tier: string; division: string; leaguePoints: number } }
  }
  userTag?: { recentData?: { kda: number; selectWins: number; selectLosses: number } }
}
interface Snapshot {
  phase?: string
  mySubteamId?: number
  subteams?: { subteamId: number; players: Player[] }[]
}
const state = ref<Snapshot>({})
const enemies = computed(() =>
  state.value.phase === 'InProgress'
    ? (state.value.subteams ?? [])
        .filter(t => t.subteamId !== state.value.mySubteamId)
        .flatMap(t => t.players)
        .slice(0, 5)
    : []
)
let stop: UnlistenFn | undefined
let disposed = false
onMounted(async () => {
  void getCurrentWindow()
    .setEffects({ effects: [Effect.Acrylic], color: [20, 30, 46, 155] })
    .catch(() => {})
  stop = await listen<Snapshot>('enemy-board-data', e => {
    state.value = e.payload ?? {}
  })
  if (disposed) {
    stop()
    return
  }
  state.value = (await invoke<Snapshot>('enemy_board_snapshot')) ?? {}
})
onUnmounted(() => {
  disposed = true
  stop?.()
})
const tiers: Record<string, string> = {
  IRON: '黑铁',
  BRONZE: '青铜',
  SILVER: '白银',
  GOLD: '黄金',
  PLATINUM: '铂金',
  EMERALD: '翡翠',
  DIAMOND: '钻石',
  MASTER: '大师',
  GRANDMASTER: '宗师',
  CHALLENGER: '王者'
}
function rank(p: Player) {
  const r = p.rank?.queueMap?.RANKED_SOLO_5x5
  return r && tiers[r.tier] ? `${tiers[r.tier]} ${r.division} · ${r.leaguePoints} LP` : '暂无段位'
}
function recent(p: Player) {
  const r = p.userTag?.recentData
  const total = (r?.selectWins ?? 0) + (r?.selectLosses ?? 0)
  return total ? `${Math.round((r!.selectWins / total) * 100)}% · ${total} 场` : '暂无战绩'
}
</script>

<template>
  <main class="glass">
    <header>
      <div>
        <span class="eyebrow">OPPONENT INTEL</span>
        <h1>敌方看板 <i></i></h1>
      </div>
      <span class="key">按住 NUM +<small>松开隐藏</small></span>
    </header>
    <div class="labels"><span>本局对手</span><span>单双排段位</span><span>近期胜率</span></div>
    <section v-for="(p, index) in enemies" :key="p.summoner.puuid || index" class="player">
      <div class="identity">
        <img
          class="portrait"
          :src="`http://asset.localhost/champion/${p.championId}`"
          :alt="`英雄 ${p.championId}`"
        />
        <div>
          <strong>{{ p.summoner.gameName || '身份暂不可用' }}</strong
          ><small>{{ p.summoner.tagLine ? `#${p.summoner.tagLine}` : '等待客户端数据' }}</small>
        </div>
      </div>
      <span class="rank">{{ p.isLoading ? '加载中…' : rank(p) }}</span
      ><span class="record">{{ p.isLoading ? '加载中…' : recent(p) }}</span>
    </section>
    <div v-if="!enemies.length" class="empty">正在等待本局敌方信息…</div>
    <footer><span>LCU · 本局公开身份</span><span>鼠标穿透 · 不抢焦点</span></footer>
  </main>
</template>

<style>
* {
  box-sizing: border-box;
}
html,
body,
#app {
  margin: 0;
  background: transparent !important;
  overflow: hidden;
}
body {
  font-family: 'Segoe UI', 'Microsoft YaHei', sans-serif;
  color: #edf4ff;
  user-select: none;
  pointer-events: none;
}
.glass {
  margin: 8px;
  padding: 22px 26px 16px;
  border: 1px solid #d0e5ff38;
  border-radius: 22px;
  background: linear-gradient(130deg, #26354cdc, #111b2ae8);
  backdrop-filter: blur(28px) saturate(140%);
  box-shadow:
    inset 0 1px 0 #ffffff24,
    0 8px 28px #00000045;
}
header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 19px;
}
.eyebrow {
  color: #8facc7;
  letter-spacing: 3px;
  font-size: 9px;
}
h1 {
  font-size: 23px;
  font-weight: 600;
  margin: 5px 0 0;
  letter-spacing: 2px;
}
i {
  display: inline-block;
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: #72e5c5;
  vertical-align: middle;
}
.key {
  padding: 7px 12px;
  border: 1px solid #ffffff20;
  border-radius: 9px;
  font-size: 11px;
  color: #c2d4e5;
  text-align: center;
}
small {
  display: block;
  font-size: 10px;
  color: #8a9fb6;
  margin-top: 3px;
}
.labels,
.player {
  display: grid;
  grid-template-columns: 1.5fr 1fr 0.8fr;
  align-items: center;
  gap: 12px;
}
.labels {
  color: #91a5bc;
  font-size: 10px;
  padding-bottom: 8px;
}
.player {
  height: 47px;
  border-top: 1px solid #ffffff0e;
  font-size: 12px;
}
.identity {
  display: flex;
  align-items: center;
  gap: 11px;
  min-width: 0;
}
.identity > div {
  min-width: 0;
}
strong {
  display: block;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-weight: 500;
}
.portrait {
  width: 32px;
  max-width: 32px;
  flex-shrink: 0;
  object-fit: cover;
  min-width: 32px;
  height: 32px;
  display: grid;
  place-items: center;
  background: #7289b325;
  border: 1px solid #ffffff15;
  border-radius: 9px;
  font-size: 10px;
  color: #b3c9df;
}
.rank {
  color: #cbd7e5;
}
.record {
  color: #8ce0ce;
  font-variant-numeric: tabular-nums;
}
footer {
  display: flex;
  justify-content: space-between;
  border-top: 1px solid #ffffff12;
  padding-top: 13px;
  margin-top: 8px;
  color: #71869e;
  font-size: 9px;
}
.empty {
  height: 235px;
  display: grid;
  place-items: center;
  color: #9cb0c8;
  font-size: 13px;
}
</style>
