import { useEffect, useMemo, useState } from 'react'
import { useSearchParams } from 'react-router-dom'
import { invoke } from '@tauri-apps/api/core'
import { api, UsageRecord, WorkSession, ProjectSlice } from '../utils/api'
import './WorkTimeline.css'

type RangeKey = 'today' | 'week' | 'custom'

type ProjectSlicePct = ProjectSlice & { pct: number }

function todayStr(): string {
  const d = new Date()
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(
    d.getDate(),
  ).padStart(2, '0')}`
}

function weekStr(): { start: string; end: string } {
  const now = new Date()
  const day = (now.getDay() + 6) % 7
  const start = new Date(now)
  start.setDate(now.getDate() - day)
  const end = new Date(start)
  end.setDate(start.getDate() + 6)
  const fmt = (d: Date) =>
    `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(
      d.getDate(),
    ).padStart(2, '0')}`
  return { start: fmt(start), end: fmt(end) }
}

function fmtDuration(total: number): string {
  const h = Math.floor(total / 3600)
  const m = Math.floor((total % 3600) / 60)
  const s = total % 60
  if (h > 0) return `${h}h ${m}m`
  if (m > 0) return `${m}m ${s}s`
  return `${s}s`
}

function fmtClock(iso: string): string {
  if (!iso) return ''
  const t = iso.split('T')[1]
  if (!t) return iso
  return t.slice(0, 5)
}

function projectColor(name: string): string {
  const palette = [
    '#8b5cf6', '#ec4899', '#ef4444', '#f97316', '#eab308',
    '#22c55e', '#14b8a6', '#0ea5e9', '#6366f1', '#a855f7',
  ]
  let h = 0
  for (let i = 0; i < name.length; i++) h = (h * 31 + name.charCodeAt(i)) >>> 0
  return palette[h % palette.length]
}

function parseDate(s: string) {
  const [y, m, d] = s.split('-').map(Number)
  return new Date(y, m - 1, d)
}

function WorkTimeline() {
  const [searchParams] = useSearchParams()
  const urlStart = searchParams.get('start')
  const urlEnd = searchParams.get('end')
  const urlMode = useMemo(() => {
    if (!urlStart || !urlEnd) return null
    const dateRe = /^\d{4}-\d{2}-\d{2}$/
    if (!dateRe.test(urlStart) || !dateRe.test(urlEnd)) return null
    const sT = parseDate(urlStart).getTime()
    const eT = parseDate(urlEnd).getTime()
    if (isNaN(sT) || isNaN(eT) || sT > eT) return null
    const days = Math.round((eT - sT) / 86400000) + 1
    return { start: urlStart, end: urlEnd, days }
  }, [urlStart, urlEnd])

  const today = todayStr()
  const initialStart = urlMode ? urlMode.start : today
  const initialEnd = urlMode ? urlMode.end : today
  const initialPreset: RangeKey = urlMode ? 'custom' : 'today'

  const [rangeKey, setRangeKey] = useState<RangeKey>(initialPreset)
  const [customStart, setCustomStart] = useState(initialStart)
  const [customEnd, setCustomEnd] = useState(initialEnd)
  const [sessions, setSessions] = useState<WorkSession[]>([])
  const [loading, setLoading] = useState(false)
  const [expanded, setExpanded] = useState<number | null>(null)
  const [sessionRecords, setSessionRecords] = useState<Record<number, UsageRecord[]>>({})
  const [editing, setEditing] = useState<{ rid: number; init: string } | null>(null)

  const range = useMemo(() => {
    if (rangeKey === 'today') return { start: todayStr(), end: todayStr() }
    if (rangeKey === 'week') return weekStr()
    return { start: customStart, end: customEnd }
  }, [rangeKey, customStart, customEnd])

  const loadSessions = async () => {
    setLoading(true)
    try {
      const data = await api.getWorkSessions(range.start, range.end)
      setSessions(data)
      setSessionRecords({})
      setExpanded(null)
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    loadSessions()
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [range.start, range.end])

  const expandSession = async (idx: number) => {
    if (expanded === idx) {
      setExpanded(null)
      return
    }
    setExpanded(idx)
    if (sessionRecords[idx]) return
    const s = sessions[idx]
    if (!s) return
    const sd = s.start_time.slice(0, 10)
    const ed = s.end_time.slice(0, 10)
    try {
      const all = await invoke<UsageRecord[]>('get_records_for_date_range', {
        startDate: sd,
        endDate: ed,
        limit: 1000,
      })
      const st = new Date(s.start_time).getTime()
      const et = new Date(s.end_time).getTime()
      const inSess = all.filter((r) => {
        const rs = new Date(r.start_time).getTime()
        const re = new Date(r.end_time).getTime()
        return !(re < st || rs > et)
      })
      setSessionRecords((m) => ({ ...m, [idx]: inSess }))
    } catch {
      setSessionRecords((m) => ({ ...m, [idx]: [] }))
    }
  }

  const onSaveProject = async (rid: number, project: string) => {
    const ok = await api.setRecordProject(rid, project)
    setEditing(null)
    if (ok) await loadSessions()
  }

  const onClearProject = async (rid: number) => {
    await api.clearRecordProject(rid)
    setEditing(null)
    await loadSessions()
  }

  const totalSec = sessions.reduce((a, s) => a + s.total_seconds, 0)

  return (
    <div className="worktimeline-page">
      <div className="wt-header">
        <h1>工作时间线</h1>
        <div className="wt-summary">
          共 <strong>{sessions.length}</strong> 个会话，累计 <strong>{fmtDuration(totalSec)}</strong>
        </div>
      </div>

      <div className="wt-range">
        <div className="range-tabs">
          <button
            className={rangeKey === 'today' ? 'active' : ''}
            onClick={() => setRangeKey('today')}
          >
            今天
          </button>
          <button
            className={rangeKey === 'week' ? 'active' : ''}
            onClick={() => setRangeKey('week')}
          >
            本周
          </button>
          <button
            className={rangeKey === 'custom' ? 'active' : ''}
            onClick={() => setRangeKey('custom')}
          >
            自定义
          </button>
        </div>
        {rangeKey === 'custom' && (
          <div className="range-custom">
            <input
              type="date"
              value={customStart}
              max={todayStr()}
              onChange={(e) => setCustomStart(e.target.value)}
            />
            <span>至</span>
            <input
              type="date"
              value={customEnd}
              max={todayStr()}
              onChange={(e) => setCustomEnd(e.target.value)}
            />
          </div>
        )}
        <div className="range-hint">
          {range.start} ~ {range.end}
        </div>
      </div>

      <div className="wt-list">
        {loading && <div className="empty-state">加载中...</div>}
        {!loading && sessions.length === 0 && (
          <div className="empty-state">
            <div style={{ fontSize: 48 }}>🕐</div>
            <p>该时段暂无工作记录</p>
          </div>
        )}

        {sessions.map((s: WorkSession, idx: number) => {
          const totalPct = 100
          const projs: ProjectSlicePct[] = s.projects
            .filter((p: ProjectSlice) => p.seconds > 0)
            .map((p: ProjectSlice) => ({ ...p, pct: (p.seconds / Math.max(s.total_seconds, 1)) * totalPct }))
          return (
            <div key={idx} className={`wt-session ${expanded === idx ? 'open' : ''}`}>
              <div className="wt-session-main" onClick={() => expandSession(idx)}>
                <div
                  className="wt-session-dot"
                  style={{ background: projectColor(s.main_project) }}
                />
                <div className="wt-session-time">
                  {fmtClock(s.start_time)} – {fmtClock(s.end_time)}
                  <div className="wt-session-duration">{fmtDuration(s.total_seconds)}</div>
                </div>
                <div className="wt-session-project">
                  <div className="wt-session-project-name">
                    <span
                      className="wt-tag"
                      style={{ borderColor: projectColor(s.main_project) }}
                    >
                      {s.main_project}
                    </span>
                  </div>
                  <div className="wt-distbar">
                    {projs.map((p: ProjectSlicePct, i: number) => (
                      <div
                        key={i}
                        className="wt-distbar-seg"
                        style={{
                          width: `${p.pct}%`,
                          background: projectColor(p.name),
                        }}
                        title={`${p.name} ${fmtDuration(p.seconds)}`}
                      />
                    ))}
                  </div>
                  <div className="wt-session-count">{s.record_count} 条记录 · 点击展开</div>
                </div>
              </div>

              {expanded === idx && (
                <div className="wt-session-records">
                  {sessionRecords[idx] && sessionRecords[idx].length === 0 && (
                    <div className="wt-hint">会话内无明细记录</div>
                  )}
                  {!sessionRecords[idx] && <div className="wt-hint">加载明细中...</div>}
                  {sessionRecords[idx]?.map((r) => (
                    <div key={r.id} className="wt-rec">
                      <div className="wt-rec-time">
                        {fmtClock(r.start_time)}
                      </div>
                      <div className="wt-rec-body">
                        <div className="wt-rec-app">
                          {r.process_name} · {fmtDuration(r.duration_seconds)}
                        </div>
                        <div className="wt-rec-title">{r.window_title || '(无标题)'}</div>
                      </div>
                      <div className="wt-rec-actions">
                        {(() => {
                          const isEditingThis = editing?.rid === r.id
                          const edit = editing
                          if (isEditingThis) {
                            return (
                              <input
                                className="wt-rec-input"
                                autoFocus
                                defaultValue={edit!.init}
                                onBlur={(e) => onSaveProject(r.id, e.target.value.trim())}
                                onKeyDown={(e) => {
                                  if (e.key === 'Enter')
                                    onSaveProject(r.id, (e.target as HTMLInputElement).value.trim())
                                  if (e.key === 'Escape') setEditing(null)
                                }}
                              />
                            )
                          }
                          return (
                            <button
                              className="wt-rec-btn"
                              onClick={() =>
                                setEditing({ rid: r.id, init: '' })
                              }
                            >
                              改归属
                            </button>
                          )
                        })()}
                        <button
                          className="wt-rec-btn wt-rec-btn-ghost"
                          onClick={() => onClearProject(r.id)}
                          title="清除手动项目归属"
                        >
                          清除
                        </button>
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </div>
          )
        })}
      </div>
    </div>
  )
}

export default WorkTimeline
