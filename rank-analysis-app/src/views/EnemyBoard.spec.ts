import { mount, flushPromises } from '@vue/test-utils'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import EnemyBoard from './EnemyBoard.vue'

const mocks = vi.hoisted(() => ({
  invoke: vi.fn(),
  listen: vi.fn(),
  stop: vi.fn(),
  effects: vi.fn()
}))
vi.mock('@tauri-apps/api/core', () => ({ invoke: mocks.invoke }))
vi.mock('@tauri-apps/api/event', () => ({ listen: mocks.listen }))
vi.mock('@tauri-apps/api/window', () => ({
  Effect: { Acrylic: 'acrylic' },
  getCurrentWindow: () => ({ setEffects: mocks.effects })
}))
const player = (name: string) => ({
  championId: 1,
  isLoading: false,
  summoner: { gameName: name, tagLine: 'TEST', puuid: name },
  userTag: { recentData: { selectWins: 6, selectLosses: 4 } }
})
beforeEach(() => {
  vi.clearAllMocks()
  mocks.listen.mockResolvedValue(mocks.stop)
  mocks.effects.mockResolvedValue(undefined)
})
describe('enemy hold board', () => {
  it('shows only opponents even when the player is on team two', async () => {
    mocks.invoke.mockResolvedValue({
      phase: 'InProgress',
      mySubteamId: 2,
      subteams: [
        { subteamId: 1, players: [player('敌方')] },
        { subteamId: 2, players: [player('己方')] }
      ]
    })
    const wrapper = mount(EnemyBoard)
    await flushPromises()
    expect(wrapper.text()).toContain('敌方')
    expect(wrapper.text()).not.toContain('己方')
    expect(wrapper.text()).toContain('60% · 10 场')
    wrapper.unmount()
    expect(mocks.stop).toHaveBeenCalled()
  })
  it('does not render player identities during champion selection', async () => {
    mocks.invoke.mockResolvedValue({
      phase: 'ChampSelect',
      mySubteamId: 1,
      subteams: [{ subteamId: 2, players: [player('隐藏身份')] }]
    })
    const wrapper = mount(EnemyBoard)
    await flushPromises()
    expect(wrapper.text()).not.toContain('隐藏身份')
    wrapper.unmount()
  })
  it('clears the previous match when a null snapshot arrives', async () => {
    mocks.invoke.mockResolvedValue(null)
    const wrapper = mount(EnemyBoard)
    await flushPromises()
    expect(wrapper.findAll('.player')).toHaveLength(0)
    expect(wrapper.text()).toContain('等待本局敌方信息')
    wrapper.unmount()
  })
})
